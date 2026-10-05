# Smart Migrate API and signaling boundary

The planned service uses Java 21, Spring Boot, PostgreSQL, Redis, authenticated HTTPS/WSS, OpenAPI, and Flyway migrations. It will authorize signaling and mint short-lived session/TURN credentials; it must never store desktop frames, raw clipboard contents, or unapproved file contents.
