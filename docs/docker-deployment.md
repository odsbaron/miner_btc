# Docker deployment

Local dashboard deployment:

```bash
docker compose up --build -d
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/api/status
open http://127.0.0.1:8080/
```

Stop:

```bash
docker compose down
```

Security notes:

- The container runs as a non-root user.
- Pool passwords are not baked into the image.
- The default compose file binds only to `127.0.0.1:8080`.
- Hardware adapters are dry-run abstractions until real cgminer/Bitaxe/Avalon APIs are explicitly configured.

Umbrel-style metadata lives in `deploy/umbrel/umbrel-app.yml` and can be moved into a community app store repo when publishing.
