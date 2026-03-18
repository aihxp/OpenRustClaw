"""Agentic RAG pipeline workflow."""

import hashlib
import json
import logging
import threading
from typing import Annotated, Any, Dict, List, Literal, Optional, Sequence, TypedDict

from langchain_core.documents import Document
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

from ..memory_bridge import MemoryBridge
from ..workflow_contract import get_configurable_value

logger = logging.getLogger(__name__)

STOPWORDS = {
    "a",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "by",
    "for",
    "from",
    "how",
    "i",
    "in",
    "is",
    "it",
    "of",
    "on",
    "or",
    "that",
    "the",
    "this",
    "to",
    "was",
    "what",
    "when",
    "where",
    "which",
    "who",
    "why",
    "with",
}


class InMemoryRagStore:
    """Minimal deterministic document store for sidecar RAG workflows."""

    def __init__(self) -> None:
        self._collections: Dict[str, List[Document]] = {}
        self._lock = threading.RLock()

    def store(self, collection_name: str, chunks: List[Document]) -> int:
        with self._lock:
            self._collections[collection_name] = [chunk for chunk in chunks]
            return len(chunks)

    def load(self, collection_name: str) -> List[Document]:
        with self._lock:
            return [chunk for chunk in self._collections.get(collection_name, [])]


RAG_STORE = InMemoryRagStore()


class RAGState(TypedDict):
    """State for the RAG pipeline graph."""

    messages: Annotated[Sequence[BaseMessage], add_messages]
    documents: List[Document]
    chunks: List[Document]
    embeddings: List[List[float]]
    query: str
    retrieved_docs: List[Document]
    answer: Optional[str]
    sources: List[Dict[str, Any]]
    collection_name: str
    context_budget_chars: int
    status: Literal["pending", "ingesting", "indexing", "querying", "completed", "error"]
    error: Optional[str]


def create_default_state() -> RAGState:
    """Create default initial state."""
    return {
        "messages": [],
        "documents": [],
        "chunks": [],
        "embeddings": [],
        "query": "",
        "retrieved_docs": [],
        "answer": None,
        "sources": [],
        "collection_name": "rag_documents",
        "context_budget_chars": 6000,
        "status": "pending",
        "error": None,
    }


class DocumentIngestionNode:
    """Node for ingesting documents from various sources."""

    def __init__(self) -> None:
        self.name = "document_ingestion"

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Ingest documents from the input."""
        logger.debug("Running document_ingestion node")

        try:
            # Documents can come from state or config
            documents = state.get("documents", [])
            collection_name = state.get("collection_name") or "rag_documents"
            context_budget_chars = state.get("context_budget_chars") or 6000

            if not documents:
                # Try to load from config
                doc_source = get_configurable_value(config, "doc_source")
                configured_collection = get_configurable_value(config, "collection_name")
                configured_budget = get_configurable_value(config, "context_budget_chars")
                if isinstance(configured_collection, str) and configured_collection.strip():
                    collection_name = configured_collection.strip()
                if isinstance(configured_budget, int) and configured_budget > 0:
                    context_budget_chars = configured_budget
                if doc_source:
                    documents = await self._load_documents(doc_source)

            if not documents:
                # Check for raw content in messages
                messages = state.get("messages", [])
                for msg in messages:
                    if isinstance(msg, HumanMessage):
                        content = msg.content
                        if isinstance(content, str) and len(content) > 100:
                            # Treat as a document
                            documents.append(
                                Document(
                                    page_content=content,
                                    metadata={"source": "user_input", "type": "text"},
                                )
                            )

            query = state.get("query", "")
            if not query:
                messages = state.get("messages", [])
                for msg in reversed(messages):
                    if isinstance(msg, HumanMessage) and isinstance(msg.content, str):
                        query = msg.content
                        break

            logger.info(f"Ingested {len(documents)} documents")

            return {
                "documents": documents,
                "query": query,
                "collection_name": collection_name,
                "context_budget_chars": context_budget_chars,
                "status": "ingesting" if documents else "querying",
                "error": None,
            }

        except Exception as e:
            logger.exception("Document ingestion failed")
            return {
                "error": str(e),
                "status": "error",
            }

    async def _load_documents(self, source: Dict[str, Any]) -> List[Document]:
        """Load documents from a source configuration."""
        source_type = source.get("type")

        if source_type == "text":
            return [Document(
                page_content=source.get("content", ""),
                metadata=source.get("metadata", {}),
            )]

        elif source_type == "file":
            # In production, use appropriate loaders
            file_path = source.get("path", "")
            if file_path.endswith(".txt"):
                try:
                    # Note: In production, use async file operations
                    with open(file_path, "r", encoding="utf-8") as f:
                        content = f.read()
                    return [Document(
                        page_content=content,
                        metadata={"source": file_path, "type": "file"},
                    )]
                except Exception as e:
                    logger.error(f"Failed to load file {file_path}: {e}")
                    return []

        elif source_type == "web":
            # In production, use web loaders
            url = source.get("url", "")
            logger.info(f"Would fetch from URL: {url}")
            return []

        return []


class ChunkingNode:
    """Node for chunking documents into smaller pieces."""

    def __init__(
        self,
        chunk_size: int = 1000,
        chunk_overlap: int = 200,
    ) -> None:
        self.name = "chunking"
        self.chunk_size = chunk_size
        self.chunk_overlap = chunk_overlap
        self._text_splitter: Optional[Any] = None

    def _get_text_splitter(self) -> Any:
        """Get or create text splitter."""
        if self._text_splitter is None:
            try:
                from langchain_text_splitters import RecursiveCharacterTextSplitter
                self._text_splitter = RecursiveCharacterTextSplitter(
                    chunk_size=self.chunk_size,
                    chunk_overlap=self.chunk_overlap,
                    length_function=len,
                    separators=["\n\n", "\n", ". ", " ", ""],
                )
            except Exception as e:
                logger.error(f"Failed to create text splitter: {e}")
                self._text_splitter = None
        return self._text_splitter

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Chunk documents into smaller pieces."""
        logger.debug("Running chunking node")

        try:
            documents = state.get("documents", [])
            if not documents:
                return {"chunks": [], "error": "No documents to chunk"}

            text_splitter = self._get_text_splitter()
            chunks: List[Document] = []

            if text_splitter is None:
                # Fallback: simple chunking
                for doc in documents:
                    content = doc.page_content
                    for i in range(0, len(content), self.chunk_size - self.chunk_overlap):
                        chunk_text = content[i:i + self.chunk_size]
                        chunks.append(Document(
                            page_content=chunk_text,
                            metadata={
                                **doc.metadata,
                                "chunk_index": i // (self.chunk_size - self.chunk_overlap),
                            },
                        ))
            else:
                for doc in documents:
                    doc_chunks = text_splitter.split_documents([doc])
                    for idx, chunk in enumerate(doc_chunks):
                        chunk.metadata["chunk_index"] = idx
                    chunks.extend(doc_chunks)

            logger.info(f"Created {len(chunks)} chunks from {len(documents)} documents")

            return {
                "chunks": chunks,
                "status": "indexing",
            }

        except Exception as e:
            logger.exception("Chunking failed")
            return {
                "error": str(e),
                "status": "error",
            }


class EmbeddingNode:
    """Node for generating embeddings for chunks."""

    def __init__(self, embedding_model: Optional[str] = None) -> None:
        self.name = "embedding"
        self.embedding_model = embedding_model or "text-embedding-3-small"
        self._embeddings: Optional[Any] = None

    def _get_embeddings(self) -> Any:
        """Get or create embeddings instance."""
        if self._embeddings is None:
            try:
                from langchain_openai import OpenAIEmbeddings
                self._embeddings = OpenAIEmbeddings(
                    model=self.embedding_model,
                )
            except Exception as e:
                logger.error(f"Failed to initialize embeddings: {e}")
                self._embeddings = None
        return self._embeddings

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Generate embeddings for chunks."""
        logger.debug("Running embedding node")

        try:
            chunks = state.get("chunks", [])
            if not chunks:
                return {"embeddings": [], "error": "No chunks to embed"}

            embeddings = self._get_embeddings()
            embedding_vectors: List[List[float]] = []

            if embeddings:
                texts = [chunk.page_content for chunk in chunks]
                embedding_vectors = await embeddings.aembed_documents(texts)
            else:
                # Mock embeddings
                embedding_vectors = [[0.0] * 1536 for _ in chunks]

            # Store embeddings in chunk metadata
            for chunk, embedding in zip(chunks, embedding_vectors):
                chunk.metadata["embedding"] = embedding

            logger.info(f"Generated {len(embedding_vectors)} embeddings")

            return {
                "embeddings": embedding_vectors,
                "chunks": chunks,
            }

        except Exception as e:
            logger.exception("Embedding generation failed")
            return {
                "error": str(e),
                "status": "error",
            }


class StorageNode:
    """Node for storing chunks and embeddings in vector store."""

    def __init__(self, collection_name: str = "rag_documents") -> None:
        self.name = "storage"
        self.collection_name = collection_name
        self._vectorstore: Optional[Any] = None
        self.memory_bridge = MemoryBridge.from_env()

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Store chunks in vector store."""
        logger.debug("Running storage node")

        try:
            chunks = state.get("chunks", [])
            collection_name = state.get("collection_name") or self.collection_name
            if not chunks:
                return {
                    "status": "querying",
                    "collection_name": collection_name,
                    "error": None,
                }

            stored_count = await self._store_chunks(collection_name, chunks)

            return {
                "status": "querying",
                "collection_name": collection_name,
                "stored_chunks": stored_count,
                "error": None,
            }

        except Exception as e:
            logger.exception("Storage failed")
            return {
                "error": str(e),
                "status": "error",
            }

    async def _store_chunks(self, collection_name: str, chunks: List[Document]) -> int:
        """Store chunks in the vector database."""
        logger.info(f"Storing {len(chunks)} chunks in collection: {collection_name}")

        # Generate IDs for chunks
        for chunk in chunks:
            content_hash = hashlib.md5(chunk.page_content.encode()).hexdigest()[:12]
            chunk.metadata["id"] = f"chunk_{content_hash}"
            chunk.metadata.setdefault("source_id", chunk.metadata["id"])
        if self.memory_bridge is not None:
            payload = [_document_to_bridge_chunk(chunk) for chunk in chunks]
            result = await self.memory_bridge.store_rag_chunks(collection_name, payload)
            stored_chunks = result.get("stored_chunks", len(payload))
            if isinstance(stored_chunks, int):
                return stored_chunks

        return RAG_STORE.store(collection_name, chunks)


class RetrievalNode:
    """Node for retrieving relevant documents based on query."""

    def __init__(self, top_k: int = 5) -> None:
        self.name = "retrieval"
        self.top_k = top_k
        self.memory_bridge = MemoryBridge.from_env()

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Retrieve relevant documents for the query."""
        logger.debug("Running retrieval node")

        try:
            query = state.get("query", "")

            # Try to extract query from messages if not in state
            if not query:
                messages = state.get("messages", [])
                for msg in reversed(messages):
                    if isinstance(msg, HumanMessage):
                        query = msg.content
                        break

            if not query:
                return {"error": "No query provided"}

            collection_name = state.get("collection_name") or "rag_documents"
            allowed_types = _parse_allowed_source_types(config)
            max_chunks_per_source = _parse_max_chunks_per_source(config)
            min_score = _parse_min_score(config)
            effective_top_k = _parse_top_k(config, self.top_k)
            preferred_source_ids = _parse_source_ids(config, "preferred_source_ids")
            required_source_ids = _parse_source_ids(config, "required_source_ids")
            excluded_source_ids = _parse_source_ids(config, "excluded_source_ids")
            excluded_source_types = _parse_source_ids(config, "excluded_source_types")
            preferred_source_types = _parse_source_ids(config, "preferred_source_types")
            min_overlap_tokens = _parse_min_overlap_tokens(config)
            available_chunks = state.get("chunks", [])
            if not available_chunks:
                if self.memory_bridge is not None:
                    bridge_chunks = await self.memory_bridge.load_rag_chunks(collection_name)
                    available_chunks = [
                        _document_from_bridge_chunk(chunk) for chunk in bridge_chunks
                    ]
                if not available_chunks:
                    available_chunks = RAG_STORE.load(collection_name)

            # Retrieve documents
            retrieved = await self._retrieve(
                query,
                available_chunks,
                allowed_types,
                top_k=effective_top_k,
                max_chunks_per_source=max_chunks_per_source,
                min_score=min_score,
                preferred_source_ids=preferred_source_ids,
                required_source_ids=required_source_ids,
                excluded_source_ids=excluded_source_ids,
                excluded_source_types=excluded_source_types,
                preferred_source_types=preferred_source_types,
                min_overlap_tokens=min_overlap_tokens,
            )

            # Extract sources for citation
            sources = [
                {
                    "content": doc.page_content[:200],
                    "metadata": doc.metadata,
                    "score": doc.metadata.get("score", 0.0),
                }
                for doc in retrieved
            ]

            return {
                "retrieved_docs": retrieved,
                "sources": sources,
                "query": query,
                "collection_name": collection_name,
                "allowed_source_types": sorted(allowed_types) if allowed_types else [],
                "preferred_source_ids": sorted(preferred_source_ids),
                "required_source_ids": sorted(required_source_ids),
                "excluded_source_ids": sorted(excluded_source_ids),
                "excluded_source_types": sorted(excluded_source_types),
                "preferred_source_types": sorted(preferred_source_types),
                "top_k": effective_top_k,
                "max_chunks_per_source": max_chunks_per_source,
                "min_score": min_score,
                "min_overlap_tokens": min_overlap_tokens,
                "retrieval_summary": {
                    "available_chunks": len(available_chunks),
                    "retrieved_chunks": len(retrieved),
                    "retrieved_source_count": len(
                        {
                            str(doc.metadata.get("source_id", doc.metadata.get("id", "unknown")))
                            for doc in retrieved
                        }
                    ),
                    "source_ids": [
                        str(doc.metadata.get("source_id", doc.metadata.get("id", "unknown")))
                        for doc in retrieved
                    ],
                    "query_token_count": len(_normalize_text(query)),
                    "average_score": (
                        sum(float(doc.metadata.get("score", 0.0)) for doc in retrieved) / len(retrieved)
                        if retrieved
                        else 0.0
                    ),
                    "max_score": max(
                        (float(doc.metadata.get("score", 0.0)) for doc in retrieved),
                        default=0.0,
                    ),
                },
            }

        except Exception as e:
            logger.exception("Retrieval failed")
            return {
                "error": str(e),
            }

    async def _retrieve(
        self,
        query: str,
        chunks: List[Document],
        allowed_source_types: Optional[set[str]] = None,
        top_k: Optional[int] = None,
        max_chunks_per_source: Optional[int] = None,
        min_score: Optional[float] = None,
        preferred_source_ids: Optional[set[str]] = None,
        required_source_ids: Optional[set[str]] = None,
        excluded_source_ids: Optional[set[str]] = None,
        excluded_source_types: Optional[set[str]] = None,
        preferred_source_types: Optional[set[str]] = None,
        min_overlap_tokens: Optional[int] = None,
    ) -> List[Document]:
        """Retrieve relevant chunks for the query."""
        query_words = set(_normalize_text(query))
        scored_chunks: List[tuple[float, Document]] = []
        effective_top_k = top_k if isinstance(top_k, int) and top_k > 0 else self.top_k
        preferred_source_ids = preferred_source_ids or set()
        required_source_ids = required_source_ids or set()
        excluded_source_ids = excluded_source_ids or set()
        excluded_source_types = excluded_source_types or set()
        preferred_source_types = preferred_source_types or set()

        for chunk in chunks:
            source_id = str(chunk.metadata.get("source_id", chunk.metadata.get("id", "unknown")))
            chunk_type = str(
                chunk.metadata.get("source_type", chunk.metadata.get("type", "text"))
            ).strip().lower()
            if allowed_source_types and chunk_type not in allowed_source_types:
                continue
            if excluded_source_types and chunk_type in excluded_source_types:
                continue
            if required_source_ids and source_id not in required_source_ids:
                continue
            if excluded_source_ids and source_id in excluded_source_ids:
                continue

            chunk_words = set(_normalize_text(chunk.page_content))
            overlap_count = len(query_words & chunk_words)
            if min_overlap_tokens is not None and overlap_count < min_overlap_tokens:
                continue
            lexical_overlap = len(query_words & chunk_words) / max(len(query_words), 1)
            coverage = overlap_count / max(len(chunk_words), 1)
            metadata_tokens = set(
                _normalize_text(
                    " ".join(
                        str(chunk.metadata.get(key, ""))
                        for key in ("title", "source", "source_id", "path")
                    )
                )
            )
            metadata_overlap = len(query_words & metadata_tokens) / max(len(query_words), 1)
            exact_phrase = 0.2 if query.lower() in chunk.page_content.lower() else 0.0
            type_boost = 0.15 if chunk_type == "code" else 0.0
            preferred_source_boost = 0.12 if source_id in preferred_source_ids else 0.0
            preferred_type_boost = 0.08 if chunk_type in preferred_source_types else 0.0
            title_match_boost = (
                0.1
                if query.lower() in str(chunk.metadata.get("title", "")).lower()
                else 0.0
            )
            source_match_boost = (
                0.08
                if query.lower() in str(chunk.metadata.get("source", "")).lower()
                else 0.0
            )
            length_penalty = 0.05 if len(chunk.page_content) > 4000 else 0.0
            score = (
                lexical_overlap * 0.55
                + coverage * 0.2
                + metadata_overlap * 0.25
                + exact_phrase
                + type_boost
                + preferred_source_boost
                + preferred_type_boost
                + title_match_boost
                + source_match_boost
                - length_penalty
            )
            scored_chunks.append((score, chunk))

        scored_chunks.sort(
            key=lambda entry: (
                entry[0],
                str(entry[1].metadata.get("source_id", entry[1].metadata.get("id", ""))),
            ),
            reverse=True,
        )
        top_chunks: List[Document] = []
        per_source_counts: Dict[str, int] = {}
        seen_chunks: set[tuple[str, str]] = set()
        per_source_limit = max_chunks_per_source if isinstance(max_chunks_per_source, int) and max_chunks_per_source > 0 else None

        for score, chunk in scored_chunks:
            if min_score is not None and score < min_score:
                continue
            source_id = str(chunk.metadata.get("source_id", chunk.metadata.get("id", "unknown")))
            dedupe_key = (source_id, chunk.page_content.strip())
            if dedupe_key in seen_chunks:
                continue
            if per_source_limit is not None and per_source_counts.get(source_id, 0) >= per_source_limit:
                continue
            chunk.metadata["score"] = score
            chunk.metadata["rank"] = len(top_chunks) + 1
            chunk.metadata["preferred_source_match"] = source_id in preferred_source_ids
            chunk.metadata["preferred_type_match"] = chunk_type in preferred_source_types
            top_chunks.append(chunk)
            seen_chunks.add(dedupe_key)
            per_source_counts[source_id] = per_source_counts.get(source_id, 0) + 1
            if len(top_chunks) >= effective_top_k:
                break

        # Add scores to metadata
        return top_chunks


class GenerationNode:
    """Node for generating answers from retrieved documents."""

    def __init__(self, model: Optional[str] = None) -> None:
        self.name = "generation"
        self.model = model or "gpt-4"
        self._llm: Optional[Any] = None

    def _get_llm(self) -> Any:
        """Get or create LLM instance."""
        if self._llm is None:
            try:
                from langchain_openai import ChatOpenAI
                self._llm = ChatOpenAI(
                    model=self.model,
                    temperature=0.3,  # Lower for factual consistency
                )
            except Exception as e:
                logger.error(f"Failed to initialize OpenAI model: {e}")
                self._llm = None
        return self._llm

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Generate answer from retrieved documents."""
        logger.debug("Running generation node")

        try:
            query = state.get("query", "")
            retrieved_docs = state.get("retrieved_docs", [])
            sources = state.get("sources", [])
            budget = state.get("context_budget_chars") or 6000
            max_sections = _parse_max_context_sections(config)
            include_scores = _parse_include_scores(config)
            max_section_chars = _parse_max_section_chars(config)
            include_metadata = _parse_include_metadata_in_context(config)

            if not retrieved_docs:
                return {
                    "answer": "I couldn't find any relevant information to answer your question.",
                    "status": "completed",
                }

            context, citation_order = _assemble_context(
                retrieved_docs,
                budget,
                max_sections=max_sections,
                include_scores=include_scores,
                max_section_chars=max_section_chars,
                include_metadata=include_metadata,
            )

            # Create prompt
            prompt = f"""Based on the following context, please answer the question. If the answer is not in the context, say "I don't have enough information to answer this question."

Context:
{context}

Question: {query}

Please provide a clear, accurate answer based only on the context provided. Cite supporting sources inline using their source ids, for example [chunk_abc123]."""

            llm = self._get_llm()

            if llm is None:
                # Mock response
                return {
                    "answer": (
                        f"Mock answer for query: {query[:50]}... "
                        f"(based on {len(retrieved_docs)} documents; sources: {', '.join(citation_order)})"
                    ),
                    "status": "completed",
                    "context_budget_chars": budget,
                }

            messages = [
                SystemMessage(content="You are a helpful assistant that answers questions based on provided documents."),
                HumanMessage(content=prompt),
            ]

            response = await llm.ainvoke(messages)

            return {
                "answer": response.content,
                "status": "completed",
                "context_budget_chars": budget,
            }

        except Exception as e:
            logger.exception("Generation failed")
            return {
                "error": str(e),
                "status": "error",
            }


def has_error(state: RAGState) -> str:
    """Check if there's an error in the state."""
    return "error" if state.get("error") else "continue"


def should_index_or_retrieve(state: RAGState) -> str:
    """Determine if we should index documents or retrieve from an existing collection."""
    return "index" if state.get("documents") else "retrieve"


def should_query(state: RAGState) -> str:
    """Determine if a query exists for retrieval/generation."""
    return "generate" if state.get("query") else "complete"


def passthrough(state: RAGState) -> Dict[str, Any]:
    """Router node used for conditional graph branching."""
    return {}


def _normalize_text(text: str) -> List[str]:
    normalized: List[str] = []
    for raw_token in text.lower().split():
        token = raw_token.strip(".,:;!?()[]{}\"'")
        if not token or token in STOPWORDS:
            continue
        normalized.append(token)
    return normalized


def _parse_allowed_source_types(
    config: Optional[RunnableConfig],
) -> Optional[set[str]]:
    configured = get_configurable_value(config, "allowed_source_types")
    if configured is None:
        configured = get_configurable_value(config, "source_types")

    if not isinstance(configured, list):
        return None

    allowed = {
        str(value).strip().lower()
        for value in configured
        if isinstance(value, (str, int, float)) and str(value).strip()
    }
    return allowed or None


def _parse_max_chunks_per_source(config: Optional[RunnableConfig]) -> Optional[int]:
    configured = get_configurable_value(config, "max_chunks_per_source")
    if configured is None:
        return None

    try:
        value = int(configured)
    except (TypeError, ValueError):
        return None

    return value if value > 0 else None


def _parse_min_score(config: Optional[RunnableConfig]) -> Optional[float]:
    configured = get_configurable_value(config, "min_score")
    if configured is None:
        return None

    try:
        value = float(configured)
    except (TypeError, ValueError):
        return None

    return value if value >= 0.0 else None


def _parse_top_k(config: Optional[RunnableConfig], default: int) -> int:
    configured = get_configurable_value(config, "top_k")
    if configured is None:
        return default

    try:
        value = int(configured)
    except (TypeError, ValueError):
        return default

    return value if value > 0 else default


def _parse_source_ids(config: Optional[RunnableConfig], key: str) -> set[str]:
    configured = get_configurable_value(config, key)
    if not isinstance(configured, list):
        return set()

    return {
        str(value).strip()
        for value in configured
        if isinstance(value, (str, int, float)) and str(value).strip()
    }

def _parse_max_context_sections(config: Optional[RunnableConfig]) -> Optional[int]:
    configured = get_configurable_value(config, "max_context_sections")
    if configured is None:
        return None

    try:
        value = int(configured)
    except (TypeError, ValueError):
        return None

    return value if value > 0 else None


def _parse_include_scores(config: Optional[RunnableConfig]) -> bool:
    configured = get_configurable_value(config, "include_scores")
    return bool(configured) if configured is not None else False


def _parse_include_metadata_in_context(config: Optional[RunnableConfig]) -> bool:
    configured = get_configurable_value(config, "include_metadata_in_context")
    return bool(configured) if configured is not None else False


def _parse_max_section_chars(config: Optional[RunnableConfig]) -> Optional[int]:
    configured = get_configurable_value(config, "max_section_chars")
    if configured is None:
        return None

    try:
        value = int(configured)
    except (TypeError, ValueError):
        return None

    return value if value > 0 else None


def _parse_min_overlap_tokens(config: Optional[RunnableConfig]) -> Optional[int]:
    configured = get_configurable_value(config, "min_overlap_tokens")
    if configured is None:
        return None

    try:
        value = int(configured)
    except (TypeError, ValueError):
        return None

    return value if value >= 0 else None


def _assemble_context(
    retrieved_docs: List[Document],
    budget: int,
    max_sections: Optional[int] = None,
    include_scores: bool = False,
    max_section_chars: Optional[int] = None,
    include_metadata: bool = False,
) -> tuple[str, List[str]]:
    remaining = max(budget, 0)
    sections: List[str] = []
    citation_order: List[str] = []

    for doc in retrieved_docs:
        if max_sections is not None and len(sections) >= max_sections:
            break
        source_id = str(doc.metadata.get("source_id", doc.metadata.get("id", "unknown")))
        if source_id not in citation_order:
            citation_order.append(source_id)

        header = f"[{source_id}]"
        if include_scores:
            header = f"{header} score={doc.metadata.get('score', 0.0):.3f}"
        if include_metadata:
            title = str(doc.metadata.get("title", "")).strip()
            source_type = str(doc.metadata.get("source_type", doc.metadata.get("type", ""))).strip()
            meta_parts = [part for part in [title, source_type] if part]
            if meta_parts:
                header = f"{header} {' | '.join(meta_parts)}"
        content = doc.page_content.strip()
        if max_section_chars is not None and max_section_chars > 0:
            content = content[:max_section_chars].rstrip()
        section = f"{header}\n{content}"
        if not section.strip():
            continue

        if len(section) <= remaining:
            sections.append(section)
            remaining -= len(section)
            continue

        if remaining <= 32:
            break

        sections.append(section[:remaining].rstrip())
        break

    return "\n\n".join(sections), citation_order


def _document_to_bridge_chunk(doc: Document) -> Dict[str, Any]:
    metadata = dict(doc.metadata)
    chunk_id = str(metadata.get("id") or metadata.get("source_id") or "chunk_unknown")
    source_id = str(metadata.get("source_id") or chunk_id)
    chunk_index = metadata.get("chunk_index", 0)
    if not isinstance(chunk_index, int):
        try:
            chunk_index = int(chunk_index)
        except (TypeError, ValueError):
            chunk_index = 0

    return {
        "id": chunk_id,
        "source_id": source_id,
        "chunk_index": chunk_index,
        "content": doc.page_content,
        "metadata": metadata,
    }


def _document_from_bridge_chunk(chunk: Dict[str, Any]) -> Document:
    metadata = chunk.get("metadata", {})
    if not isinstance(metadata, dict):
        metadata = {}
    metadata = dict(metadata)
    metadata.setdefault("id", chunk.get("id", "chunk_unknown"))
    metadata.setdefault("source_id", chunk.get("source_id", metadata["id"]))
    metadata.setdefault("chunk_index", chunk.get("chunk_index", 0))
    return Document(
        page_content=str(chunk.get("content", "")),
        metadata=metadata,
    )


def build_rag_graph(
    chunk_size: int = 1000,
    chunk_overlap: int = 200,
    top_k: int = 5,
    model: Optional[str] = None,
    embedding_model: Optional[str] = None,
) -> StateGraph:
    """Build the agentic RAG pipeline graph.

    The graph supports two modes:
    1. Indexing mode: ingest -> chunk -> embed -> store
    2. Query mode: ingest -> chunk -> embed -> store -> retrieve -> generate

    Args:
        chunk_size: Size of text chunks
        chunk_overlap: Overlap between chunks
        top_k: Number of documents to retrieve
        model: LLM model for generation
        embedding_model: Model for generating embeddings

    Returns:
        Compiled StateGraph
    """
    # Create nodes
    ingestion = DocumentIngestionNode()
    chunking = ChunkingNode(chunk_size=chunk_size, chunk_overlap=chunk_overlap)
    embedding = EmbeddingNode(embedding_model=embedding_model)
    storage = StorageNode()
    retrieval = RetrievalNode(top_k=top_k)
    generation = GenerationNode(model=model)

    # Build graph
    workflow = StateGraph(RAGState)

    # Add nodes
    workflow.add_node("document_ingestion", ingestion)
    workflow.add_node("document_router", passthrough)
    workflow.add_node("chunking", chunking)
    workflow.add_node("embedding", embedding)
    workflow.add_node("storage", storage)
    workflow.add_node("retrieval", retrieval)
    workflow.add_node("generation", generation)

    # Add edges
    workflow.add_edge(START, "document_ingestion")

    # Error checking after ingestion
    workflow.add_conditional_edges(
        "document_ingestion",
        has_error,
        {
            "error": END,
            "continue": "document_router",
        },
    )

    workflow.add_conditional_edges(
        "document_router",
        should_index_or_retrieve,
        {
            "index": "chunking",
            "retrieve": "retrieval",
        },
    )

    workflow.add_edge("chunking", "embedding")
    workflow.add_edge("embedding", "storage")

    # Conditional from storage: query mode or just indexing
    workflow.add_conditional_edges(
        "storage",
        should_query,
        {
            "generate": "retrieval",
            "complete": END,
        },
    )

    workflow.add_conditional_edges(
        "retrieval",
        should_query,
        {
            "generate": "generation",
            "complete": END,
        },
    )
    workflow.add_edge("generation", END)

    return workflow.compile()
