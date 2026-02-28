# DISTRIBUTED_ARCHITECTURE

## Overview
The goal of making `tuliprox` a distributed application is to allow multiple identical instances (nodes) to run concurrently behind a load balancer. This enables high availability, load balancing for stream proxying, and horizontal scalability for processing and metadata retrieval.

Currently, `tuliprox` relies heavily on local state:
- Local SQLite / custom BPlusTree databases (`backend/src/repository/bplustree.rs`) for metadata, playlists, and library storage.
- In-memory tracking (`ActiveUserManager`, `ActiveProviderManager`, `SharedStreamManager`) for user connections, sessions, stream sharing, and rate limiting (via `tower-governor`).
- Local configuration files and cache directories.

To transition to a distributed architecture, we will decouple the local state into a centralized and shared state utilizing **PostgreSQL** and **Redis**.

## Core Architectural Changes

### 1. Database Migration: BPlusTree/SQLite to PostgreSQL
Currently, metadata, playlist caches, and library data are stored locally using custom BPlusTree files (`m3u_*.db`, `xtream_*.db`, etc.).
- **Goal**: Move persistent and shared state into PostgreSQL.
- **Why PostgreSQL**: It provides strong consistency, transactional updates, and can easily store structured metadata and JSON blobs.
- **Implementation Tasks**:
  - Integrate an asynchronous database driver (e.g., `sqlx` or `tokio-postgres`).
  - Define PostgreSQL schemas for:
    - Target playlists and mapping results.
    - Metadata and EPG data.
    - User configuration/bouquets.
  - Implement a PostgreSQL-backed repository layer extending or replacing the current `bplustree.rs` / `sqlite` repositories.
  - *Note*: Large binary blobs (like the actual stream data or images) should still be handled via distributed caching or object storage, but metadata goes to Postgres.

### 2. Rate Limiting and Session Tracking: In-memory to Redis
Rate limiting is currently handled by `tower-governor` using local memory, and session/connection limits are tracked locally via `ActiveUserManager`.
- **Goal**: Enforce global connection limits and rate limiting across all nodes using Redis.
- **Why Redis**: It offers high-throughput, low-latency operations perfect for distributed counters, rate limiting, and session state.
- **Implementation Tasks**:
  - Add `redis` crate support with asynchronous multiplexing (e.g., `redis::aio::MultiplexedConnection` or `bb8-redis` connection pooling).
  - Replace the local memory store in `tower_governor` with a Redis-backed store. (Depending on the `tower-governor` version, this may require writing a custom storage backend or utilizing `governor-redis`).
  - Migrate `ActiveUserManager` to use Redis hashes/sets to track active user connections globally.
  - Ensure operations checking `max_connections` increment and decrement atomic counters in Redis (using lua scripts if necessary to avoid race conditions).

### 3. Distributed Stream Sharing & Provider Limits
`tuliprox` shares live stream connections to upstream providers to reduce load (`SharedStreamManager`) and tracks upstream provider connection limits (`ActiveProviderManager`).
- **Goal**: Coordinate stream pulling and provider limits across nodes.
- **Why Redis**: Fast pub/sub and atomic operations.
- **Implementation Tasks**:
  - **Provider Limits**: Migrate `ActiveProviderManager` to use Redis to track how many connections are currently open to a specific upstream provider.
  - **Stream Sharing**:
    - When a client requests a stream, the node checks Redis to see if another node is already pulling this stream.
    - If so, the requesting node needs a mechanism to receive the stream data from the node pulling it (e.g., via internal proxying/HTTP streaming between nodes or a pub/sub mechanism if stream chunks are small enough, though internal HTTP streaming is preferred for bandwidth).
    - If not, the current node pulls the stream from the provider and registers itself in Redis as the "owner" of the stream.

### 4. Background Workers & Scheduled Tasks
`tuliprox` runs background tasks for metadata updates (TMDB, FFprobe) and playlist updates.
- **Goal**: Prevent multiple nodes from executing the exact same playlist update or metadata probe simultaneously.
- **Implementation Tasks**:
  - Implement a distributed locking mechanism using Redis (e.g., Redlock or simple `SET NX PX`).
  - When a cron schedule triggers a `PlaylistUpdate` or `LibraryScan`, the node attempts to acquire the lock. Only the node holding the lock performs the update and persists the changes to PostgreSQL.

### 5. Config Management and Hot Reloading
- **Goal**: Ensure all nodes operate on the same configuration.
- **Implementation Tasks**:
  - Store configuration files in a shared filesystem or database.
  - If stored in PostgreSQL, nodes can subscribe to a Redis Pub/Sub channel (e.g., `config_updates`) to trigger hot-reloads simultaneously when the web UI saves new configurations.

## Recommended Steps for Implementation

The transition should be done incrementally to ensure stability:

**Step 1: Introduce Redis for Rate Limiting and User Sessions (Easiest & Highest ROI)**
- Add the `redis` dependency.
- Create a `RedisClient` service.
- Refactor `ActiveUserManager` to sync active connections with Redis.
- Update `tower-governor` configuration to use a Redis store.
- **Benefit**: Allows you to run multiple proxy nodes immediately, correctly tracking user connections globally.

**Step 2: Distributed Locking for Scheduled Tasks**
- Implement Redis-based distributed locks.
- Wrap scheduled tasks (`PlaylistUpdate`, `LibraryScan`) with the lock.
- **Benefit**: Prevents multiple nodes from hammering the provider at the same scheduled time.

**Step 3: Database Migration to PostgreSQL**
- Design PostgreSQL schema.
- Implement the PostgreSQL repository.
- Add a configuration toggle to switch between local `BPlusTree` and `PostgreSQL`.
- **Benefit**: Decentralizes data storage. Note that this is the most complex step as it replaces the core data layer.

**Step 4: Distributed Stream Sharing**
- Implement node-to-node stream sharing discovery via Redis.
- **Benefit**: Maximizes provider connection efficiency across the entire cluster.

## Conclusion
By replacing local state storage with PostgreSQL and utilizing Redis for high-speed tracking and locking, `tuliprox` can transition into a highly available, horizontally scalable distributed application without compromising its core features.