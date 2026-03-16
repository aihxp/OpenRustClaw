"""Agentic RAG pipeline workflow."""

import hashlib
import json
import logging
from dataclasses import dataclass
from typing import Annotated, Any, AsyncIterator, Dict, List, Literal, Optional, Sequence, TypedDict

from langchain_core.documents import Document
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

logger = logging.getLogger(__name__)


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

            if not documents:
                # Try to load from config
                doc_source = config.get("configurable", {}).get("doc_source") if config else None
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

            logger.info(f"Ingested {len(documents)} documents")

            return {
                "documents": documents,
                "status": "ingesting" if documents else "error",
                "error": None if documents else "No documents provided",
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

    async def __call__(
        self,
        state: RAGState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Store chunks in vector store."""
        logger.debug("Running storage node")

        try:
            chunks = state.get("chunks", [])
            if not chunks:
                return {"error": "No chunks to store"}

            # In production, this would store in a proper vector database
            # For now, we simulate storage
            stored_count = await self._store_chunks(chunks)

            return {
                "status": "querying",
                "error": None,
            }

        except Exception as e:
            logger.exception("Storage failed")
            return {
                "error": str(e),
                "status": "error",
            }

    async def _store_chunks(self, chunks: List[Document]) -> int:
        """Store chunks in the vector database."""
        # Placeholder - would integrate with vector store
        logger.info(f"Storing {len(chunks)} chunks in collection: {self.collection_name}")

        # Generate IDs for chunks
        for chunk in chunks:
            content_hash = hashlib.md5(chunk.page_content.encode()).hexdigest()[:12]
            chunk.metadata["id"] = f"chunk_{content_hash}"

        return len(chunks)


class RetrievalNode:
    """Node for retrieving relevant documents based on query."""

    def __init__(self, top_k: int = 5) -> None:
        self.name = "retrieval"
        self.top_k = top_k

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

            # Retrieve documents
            retrieved = await self._retrieve(query, state.get("chunks", []))

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
            }

        except Exception as e:
            logger.exception("Retrieval failed")
            return {
                "error": str(e),
            }

    async def _retrieve(self, query: str, chunks: List[Document]) -> List[Document]:
        """Retrieve relevant chunks for the query."""
        # In production, this would use vector similarity search
        # For now, use simple keyword matching as fallback

        query_words = set(query.lower().split())
        scored_chunks: List[tuple] = []

        for chunk in chunks:
            chunk_words = set(chunk.page_content.lower().split())
            score = len(query_words & chunk_words) / len(query_words) if query_words else 0
            scored_chunks.append((score, chunk))

        # Sort by score and take top_k
        scored_chunks.sort(key=lambda x: x[0], reverse=True)
        top_chunks = [chunk for score, chunk in scored_chunks[:self.top_k]]

        # Add scores to metadata
        for score, chunk in scored_chunks[:self.top_k]:
            chunk.metadata["score"] = score

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

            if not retrieved_docs:
                return {
                    "answer": "I couldn't find any relevant information to answer your question.",
                    "status": "completed",
                }

            # Build context from retrieved documents
            context = "\n\n".join([
                f"Document {i+1}:\n{doc.page_content}"
                for i, doc in enumerate(retrieved_docs)
            ])

            # Create prompt
            prompt = f"""Based on the following context, please answer the question. If the answer is not in the context, say "I don't have enough information to answer this question."

Context:
{context}

Question: {query}

Please provide a clear, accurate answer based only on the context provided."""

            llm = self._get_llm()

            if llm is None:
                # Mock response
                return {
                    "answer": f"Mock answer for query: {query[:50]}... (based on {len(retrieved_docs)} documents)",
                    "status": "completed",
                }

            messages = [
                SystemMessage(content="You are a helpful assistant that answers questions based on provided documents."),
                HumanMessage(content=prompt),
            ]

            response = await llm.ainvoke(messages)

            return {
                "answer": response.content,
                "status": "completed",
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


def should_retrieve(state: RAGState) -> str:
    """Determine if we should do retrieval or if this is just indexing."""
    query = state.get("query", "")
    messages = state.get("messages", [])

    # Check if there's a query in messages
    has_query = bool(query)
    if not has_query and messages:
        for msg in messages:
            if isinstance(msg, HumanMessage):
                has_query = True
                break

    return "retrieve" if has_query else "complete"


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
            "continue": "chunking",
        },
    )

    workflow.add_edge("chunking", "embedding")
    workflow.add_edge("embedding", "storage")

    # Conditional from storage: query mode or just indexing
    workflow.add_conditional_edges(
        "storage",
        should_retrieve,
        {
            "retrieve": "retrieval",
            "complete": END,
        },
    )

    workflow.add_edge("retrieval", "generation")
    workflow.add_edge("generation", END)

    return workflow.compile()
