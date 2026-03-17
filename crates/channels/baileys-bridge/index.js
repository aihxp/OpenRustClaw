#!/usr/bin/env node

/**
 * Baileys Bridge for OpenRustClaw
 * 
 * This Node.js process wraps the Baileys library to provide WhatsApp Web
 * integration for the OpenRustClaw Rust agent framework.
 * 
 * Communication Protocol:
 * - Messages are sent as JSON objects over stdin/stdout
 * - Each message is terminated by a newline
 * - Message types are defined by the 'type' field
 * 
 * @module baileys-bridge
 */

const { 
    default: makeWASocket, 
    DisconnectReason, 
    useMultiFileAuthState,
    fetchLatestBaileysVersion,
    makeCacheableSignalKeyStore,
    getAggregateVotesInPollMessage,
    Browsers,
    makeInMemoryStore,
    proto
} = require('@whiskeysockets/baileys');

const pino = require('pino');
const fs = require('fs');
const path = require('path');

// Configure logging - use pino for structured logging that works with Baileys
const logger = pino({ 
    level: process.env.LOG_LEVEL || 'warn',
    transport: process.env.NODE_ENV === 'development' ? { target: 'pino-pretty' } : undefined
});

// State management
let sock = null;
let store = null;
let state = {
    connection: 'close',
    qrCode: null,
    pairingCode: null,
    reconnectAttempts: 0,
    shouldReconnect: true,
    sessionPath: null,
    pairingMode: false
};

// Message queue for outgoing messages
const messageQueue = [];
let processingQueue = false;

/**
 * Send a message to the parent Rust process
 * @param {Object} message - The message object to send
 */
function sendToParent(message) {
    const json = JSON.stringify(message);
    process.stdout.write(json + '\n');
}

/**
 * Send an error message to the parent process
 * @param {string} code - Error code
 * @param {string} message - Error message
 * @param {string} [requestId] - Optional request ID for correlation
 */
function sendError(code, message, requestId = null) {
    sendToParent({
        type: 'error',
        request_id: requestId,
        code,
        message
    });
}

/**
 * Log to stderr for debugging
 * @param {...any} args - Arguments to log
 */
function log(...args) {
    console.error(...args);
}

/**
 * Initialize the WhatsApp connection
 * @param {string} sessionPath - Path to store session credentials
 * @param {boolean} pairingMode - Whether to use pairing code instead of QR
 */
async function startWhatsApp(sessionPath, pairingMode = false) {
    state.sessionPath = sessionPath;
    state.pairingMode = pairingMode;
    state.reconnectAttempts = 0;
    
    // Ensure session directory exists
    if (!fs.existsSync(sessionPath)) {
        fs.mkdirSync(sessionPath, { recursive: true });
    }

    try {
        const { state: authState, saveCreds } = await useMultiFileAuthState(sessionPath);
        
        // Create store for caching
        store = makeInMemoryStore({ logger });
        store.readFromFile(path.join(sessionPath, 'baileys_store.json'));
        
        // Save store every 10 seconds
        setInterval(() => {
            store.writeToFile(path.join(sessionPath, 'baileys_store.json'));
        }, 10000);

        const { version, isLatest } = await fetchLatestBaileysVersion();
        log(`Using Baileys v${version.join('.')}, isLatest: ${isLatest}`);

        sock = makeWASocket({
            version,
            logger,
            printQRInTerminal: false,
            auth: {
                creds: authState.creds,
                keys: makeCacheableSignalKeyStore(authState.keys, logger),
            },
            browser: Browsers.macOS('Desktop'),
            generateHighQualityLinkPreview: true,
            syncFullHistory: false,
            markOnlineOnConnect: true,
            keepAliveIntervalMs: 30000,
            connectTimeoutMs: 60000,
            defaultQueryTimeoutMs: 60000,
            retryRequestDelayMs: 250,
            maxMsgRetryCount: 5,
            fireInitQueries: true,
            shouldIgnoreJid: jid => jid?.includes('broadcast'),
            getMessage: async (key) => {
                if (store) {
                    const msg = await store.loadMessage(key.remoteJid, key.id);
                    return msg?.message || undefined;
                }
                return proto.Message.fromObject({});
            }
        });

        // Bind store to events
        store.bind(sock.ev);

        // Handle connection events
        sock.ev.on('connection.update', async (update) => {
            const { connection, lastDisconnect, qr } = update;

            if (qr) {
                state.qrCode = qr;
                sendToParent({
                    type: 'qr_code',
                    qr_code: qr
                });
            }

            if (connection === 'close') {
                const shouldReconnect = (lastDisconnect?.error?.output?.statusCode !== DisconnectReason.loggedOut);
                const statusCode = lastDisconnect?.error?.output?.statusCode;
                
                log('Connection closed due to ', lastDisconnect?.error, ', reconnecting ', shouldReconnect);
                
                state.connection = 'close';
                state.qrCode = null;
                state.pairingCode = null;
                
                sendToParent({
                    type: 'disconnected',
                    reason: lastDisconnect?.error?.message || `Status code: ${statusCode}`
                });

                // Reconnect if not logged out
                if (shouldReconnect && state.shouldReconnect) {
                    state.reconnectAttempts++;
                    const delay = Math.min(5000 * state.reconnectAttempts, 60000);
                    log(`Reconnecting in ${delay}ms (attempt ${state.reconnectAttempts})`);
                    
                    setTimeout(() => {
                        startWhatsApp(state.sessionPath, state.pairingMode);
                    }, delay);
                }
            } else if (connection === 'open') {
                state.connection = 'open';
                state.qrCode = null;
                state.pairingCode = null;
                state.reconnectAttempts = 0;
                
                log('Connection opened');
                sendToParent({ type: 'connected' });
                
                // Process any queued messages
                processMessageQueue();
            }

            // Request pairing code if in pairing mode
            if (connection === 'connecting' && pairingMode && !state.pairingCode && !sock.user) {
                try {
                    // Wait a bit for the connection to establish
                    setTimeout(async () => {
                        if (sock && !sock.user) {
                            const code = await sock.requestPairingCode('OpenRustClaw');
                            state.pairingCode = code;
                            sendToParent({
                                type: 'pairing_code',
                                code: code
                            });
                        }
                    }, 2000);
                } catch (err) {
                    log('Error requesting pairing code:', err);
                }
            }
        });

        // Handle credentials update
        sock.ev.on('creds.update', saveCreds);

        // Handle incoming messages
        sock.ev.on('messages.upsert', async (m) => {
            if (m.type === 'notify') {
                for (const msg of m.messages) {
                    try {
                        await handleIncomingMessage(msg);
                    } catch (err) {
                        log('Error handling message:', err);
                    }
                }
            }
        });

        // Handle message updates (receipts, etc.)
        sock.ev.on('message.update', async (updates) => {
            for (const update of updates) {
                log('Message update:', update);
            }
        });

        // Handle presence updates
        sock.ev.on('presence.update', (json) => {
            log('Presence update:', json);
        });

        // Handle chats updates
        sock.ev.on('chats.update', (chats) => {
            log('Chats update:', chats.length, 'chats');
        });

        // Handle groups metadata updates
        sock.ev.on('groups.update', (groups) => {
            log('Groups update:', groups.length, 'groups');
        });

    } catch (err) {
        log('Error starting WhatsApp:', err);
        sendError('START_ERROR', err.message);
    }
}

/**
 * Process an incoming message and forward to Rust
 * @param {Object} msg - The Baileys message object
 */
async function handleIncomingMessage(msg) {
    if (msg.key.fromMe) {
        return; // Ignore messages from self
    }

    const jid = msg.key.remoteJid;
    const isGroup = jid.endsWith('@g.us');
    const messageContent = msg.message;

    if (!messageContent) {
        return;
    }

    // Extract sender info
    const sender = msg.key.participant || jid;
    const senderName = msg.pushName || 'Unknown';

    // Get group info if applicable
    let groupId = null;
    let groupName = null;
    
    if (isGroup) {
        groupId = jid;
        try {
            const groupMetadata = await sock.groupMetadata(jid);
            groupName = groupMetadata.subject;
        } catch (err) {
            log('Error getting group metadata:', err);
        }
    }

    // Handle different message types
    if (messageContent.conversation || messageContent.extendedTextMessage?.text) {
        // Text message
        const text = messageContent.conversation || messageContent.extendedTextMessage?.text;
        const contextInfo = messageContent.extendedTextMessage?.contextInfo;
        
        // Extract mentions
        const mentions = contextInfo?.mentionedJid || [];
        
        // Extract quoted message
        let quotedMessage = null;
        if (contextInfo?.quotedMessage) {
            const quoted = contextInfo.quotedMessage;
            if (quoted.conversation) {
                quotedMessage = quoted.conversation;
            } else if (quoted.extendedTextMessage?.text) {
                quotedMessage = quoted.extendedTextMessage.text;
            }
        }

        sendToParent({
            type: 'message',
            id: msg.key.id,
            from: sender,
            from_name: senderName,
            content: text,
            timestamp: msg.messageTimestamp?.low || msg.messageTimestamp || Date.now() / 1000,
            is_group: isGroup,
            group_id: groupId,
            group_name: groupName,
            quoted_message: quotedMessage,
            mentions: mentions
        });

    } else if (messageContent.imageMessage) {
        handleMediaMessage(msg, 'image', messageContent.imageMessage, sender, senderName, isGroup, groupId, groupName);
    } else if (messageContent.videoMessage) {
        handleMediaMessage(msg, 'video', messageContent.videoMessage, sender, senderName, isGroup, groupId, groupName);
    } else if (messageContent.audioMessage) {
        const isVoice = messageContent.audioMessage.ptt;
        handleMediaMessage(msg, isVoice ? 'voice' : 'audio', messageContent.audioMessage, sender, senderName, isGroup, groupId, groupName);
    } else if (messageContent.documentMessage) {
        handleMediaMessage(msg, 'document', messageContent.documentMessage, sender, senderName, isGroup, groupId, groupName);
    } else if (messageContent.stickerMessage) {
        handleMediaMessage(msg, 'sticker', messageContent.stickerMessage, sender, senderName, isGroup, groupId, groupName);
    } else if (messageContent.pollCreationMessage || messageContent.pollCreationMessageV2 || messageContent.pollCreationMessageV3) {
        // Poll message - treat as text with poll options
        const poll = messageContent.pollCreationMessage || messageContent.pollCreationMessageV2 || messageContent.pollCreationMessageV3;
        const options = poll.options?.map(o => o.optionName).join(', ') || '';
        
        sendToParent({
            type: 'message',
            id: msg.key.id,
            from: sender,
            from_name: senderName,
            content: `[Poll: ${poll.name}] Options: ${options}`,
            timestamp: msg.messageTimestamp?.low || msg.messageTimestamp || Date.now() / 1000,
            is_group: isGroup,
            group_id: groupId,
            group_name: groupName,
            quoted_message: null,
            mentions: []
        });
    } else if (messageContent.locationMessage) {
        const loc = messageContent.locationMessage;
        sendToParent({
            type: 'message',
            id: msg.key.id,
            from: sender,
            from_name: senderName,
            content: `[Location: ${loc.degreesLatitude}, ${loc.degreesLongitude}] ${loc.name || ''} ${loc.address || ''}`,
            timestamp: msg.messageTimestamp?.low || msg.messageTimestamp || Date.now() / 1000,
            is_group: isGroup,
            group_id: groupId,
            group_name: groupName,
            quoted_message: null,
            mentions: []
        });
    } else if (messageContent.contactMessage || messageContent.contactsArrayMessage) {
        sendToParent({
            type: 'message',
            id: msg.key.id,
            from: sender,
            from_name: senderName,
            content: '[Contact shared]',
            timestamp: msg.messageTimestamp?.low || msg.messageTimestamp || Date.now() / 1000,
            is_group: isGroup,
            group_id: groupId,
            group_name: groupName,
            quoted_message: null,
            mentions: []
        });
    } else {
        // Unknown message type
        log('Unknown message type:', Object.keys(messageContent));
    }
}

/**
 * Handle media messages
 */
async function handleMediaMessage(msg, mediaType, mediaInfo, sender, senderName, isGroup, groupId, groupName) {
    let url = null;
    
    // Try to download media if possible
    try {
        const buffer = await downloadMediaMessage(msg, 'buffer', {}, logger);
        if (buffer) {
            // In a real implementation, you might upload this to a file server
            // and return the URL. For now, we just note it exists.
            url = `data:${mediaInfo.mimetype};base64,...`;
        }
    } catch (err) {
        log('Error downloading media:', err);
    }

    sendToParent({
        type: 'media_message',
        id: msg.key.id,
        from: sender,
        from_name: senderName,
        media: {
            media_type: mediaType,
            mime_type: mediaInfo.mimetype,
            file_name: mediaInfo.fileName,
            file_size: mediaInfo.fileLength?.low || mediaInfo.fileLength,
            caption: mediaInfo.caption,
            duration_seconds: mediaInfo.seconds,
            url: url
        },
        timestamp: msg.messageTimestamp?.low || msg.messageTimestamp || Date.now() / 1000,
        is_group: isGroup,
        group_id: groupId,
        group_name: groupName
    });
}

/**
 * Send a text message
 * @param {string} to - Recipient JID
 * @param {string} content - Message text
 * @param {string|null} replyTo - Optional message ID to reply to
 * @param {string} requestId - Request ID for correlation
 */
async function sendMessage(to, content, replyTo = null, requestId) {
    try {
        if (!sock || state.connection !== 'open') {
            throw new Error('Not connected');
        }

        const options = {};
        if (replyTo) {
            options.quoted = { key: { id: replyTo, remoteJid: to } };
        }

        const result = await sock.sendMessage(to, { text: content }, options);
        
        sendToParent({
            type: 'message_sent',
            request_id: requestId,
            message_id: result.key.id
        });
    } catch (err) {
        log('Error sending message:', err);
        sendError('SEND_ERROR', err.message, requestId);
    }
}

/**
 * Send a media message
 * @param {string} to - Recipient JID
 * @param {string} mediaType - Type of media
 * @param {string} urlOrPath - URL or file path
 * @param {string|null} caption - Optional caption
 * @param {string|null} replyTo - Optional message ID to reply to
 * @param {string} requestId - Request ID for correlation
 */
async function sendMedia(to, mediaType, urlOrPath, caption = null, replyTo = null, requestId) {
    try {
        if (!sock || state.connection !== 'open') {
            throw new Error('Not connected');
        }

        let messageContent = {};
        
        // Determine if it's a URL or file path
        const isUrl = urlOrPath.startsWith('http://') || urlOrPath.startsWith('https://');
        const isDataUrl = urlOrPath.startsWith('data:');

        switch (mediaType) {
            case 'image':
                messageContent = { 
                    image: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath },
                    caption: caption
                };
                break;
            case 'video':
                messageContent = { 
                    video: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath },
                    caption: caption,
                    gifPlayback: false
                };
                break;
            case 'audio':
                messageContent = { 
                    audio: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath },
                    mimetype: 'audio/mp4',
                    ptt: false
                };
                break;
            case 'voice':
                messageContent = { 
                    audio: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath },
                    mimetype: 'audio/ogg; codecs=opus',
                    ptt: true
                };
                break;
            case 'document':
                messageContent = { 
                    document: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath },
                    mimetype: undefined, // Will be inferred
                    fileName: caption || 'document'
                };
                break;
            case 'sticker':
                messageContent = { 
                    sticker: isUrl || isDataUrl ? { url: urlOrPath } : { url: urlOrPath }
                };
                break;
            default:
                throw new Error(`Unknown media type: ${mediaType}`);
        }

        const options = {};
        if (replyTo) {
            options.quoted = { key: { id: replyTo, remoteJid: to } };
        }

        const result = await sock.sendMessage(to, messageContent, options);
        
        sendToParent({
            type: 'message_sent',
            request_id: requestId,
            message_id: result.key.id
        });
    } catch (err) {
        log('Error sending media:', err);
        sendError('SEND_MEDIA_ERROR', err.message, requestId);
    }
}

/**
 * Process queued outgoing messages
 */
async function processMessageQueue() {
    if (processingQueue || messageQueue.length === 0) {
        return;
    }
    
    processingQueue = true;
    
    while (messageQueue.length > 0) {
        const item = messageQueue.shift();
        try {
            if (item.type === 'message') {
                await sendMessage(item.to, item.content, item.replyTo, item.requestId);
            } else if (item.type === 'media') {
                await sendMedia(item.to, item.mediaType, item.urlOrPath, item.caption, item.replyTo, item.requestId);
            }
        } catch (err) {
            log('Error processing queued message:', err);
        }
        
        // Small delay to avoid rate limiting
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    processingQueue = false;
}

/**
 * Handle messages from parent process
 */
function handleParentMessage(line) {
    try {
        const msg = JSON.parse(line);
        
        switch (msg.type) {
            case 'connect':
                startWhatsApp(msg.session_path, msg.pairing_mode);
                break;
                
            case 'send_message':
                if (state.connection === 'open') {
                    sendMessage(msg.to, msg.content, msg.reply_to, msg.request_id);
                } else {
                    // Queue message for when connection is ready
                    messageQueue.push({
                        type: 'message',
                        to: msg.to,
                        content: msg.content,
                        replyTo: msg.reply_to,
                        requestId: msg.request_id
                    });
                }
                break;
                
            case 'send_media':
                if (state.connection === 'open') {
                    sendMedia(msg.to, msg.media_type, msg.url_or_path, msg.caption, msg.reply_to, msg.request_id);
                } else {
                    messageQueue.push({
                        type: 'media',
                        to: msg.to,
                        mediaType: msg.media_type,
                        urlOrPath: msg.url_or_path,
                        caption: msg.caption,
                        replyTo: msg.reply_to,
                        requestId: msg.request_id
                    });
                }
                break;
                
            case 'ping':
                sendToParent({ type: 'pong' });
                break;
                
            case 'disconnect':
                state.shouldReconnect = false;
                if (sock) {
                    sock.end(undefined);
                }
                process.exit(0);
                break;
                
            default:
                log('Unknown message type:', msg.type);
        }
    } catch (err) {
        log('Error handling parent message:', err);
        sendError('PARSE_ERROR', err.message);
    }
}

// Set up stdin reader
let buffer = '';

process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
    buffer += chunk;
    
    // Process complete lines
    let newlineIndex;
    while ((newlineIndex = buffer.indexOf('\n')) !== -1) {
        const line = buffer.substring(0, newlineIndex).trim();
        buffer = buffer.substring(newlineIndex + 1);
        
        if (line) {
            handleParentMessage(line);
        }
    }
});

process.stdin.on('end', () => {
    log('Stdin closed, exiting');
    process.exit(0);
});

// Handle process signals
process.on('SIGINT', () => {
    log('SIGINT received, closing connection');
    state.shouldReconnect = false;
    if (sock) {
        sock.end(undefined);
    }
    process.exit(0);
});

process.on('SIGTERM', () => {
    log('SIGTERM received, closing connection');
    state.shouldReconnect = false;
    if (sock) {
        sock.end(undefined);
    }
    process.exit(0);
});

process.on('uncaughtException', (err) => {
    log('Uncaught exception:', err);
    sendError('UNCAUGHT_EXCEPTION', err.message);
});

process.on('unhandledRejection', (reason, promise) => {
    log('Unhandled rejection at:', promise, 'reason:', reason);
});

// Log startup
log('Baileys bridge started, waiting for connection command...');
