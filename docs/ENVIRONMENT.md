# Smart Migrate environment contract

Environment values are injected by the deployment platform and never committed. Add real values only to an ignored local `.env` file or secret manager.

| Variable | Scope | Purpose |
|---|---|---|
| `SMART_MIGRATE_API_BASE_URL` | Clients/Web | HTTPS API origin |
| `SMART_MIGRATE_SIGNALING_URL` | Clients | WSS signaling endpoint |
| `SMART_MIGRATE_DATABASE_URL` | API | PostgreSQL connection string |
| `SMART_MIGRATE_REDIS_URL` | API | Redis connection string |
| `SMART_MIGRATE_JWT_SIGNING_KEY` | API secret | Signing material; secret-manager only |
| `SMART_MIGRATE_TURN_SHARED_SECRET` | API/TURN secret | Ephemeral TURN credential minting |
| `SMART_MIGRATE_ALLOWED_ORIGINS` | API | Comma-separated trusted browser origins |
| `SMART_MIGRATE_LOG_LEVEL` | Services | Privacy-safe log threshold |

Production additionally requires TLS certificates, signing certificates, Android signing configuration, code-signing access, public STUN/TURN reachability, and separate secrets per environment.
