---
description: to generate docker image & tar
---

docker build --platform linux/amd64 - t stickplay-server:latest .
docker save stickplay-erver.tar