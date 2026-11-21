#!/usr/bin/env bash
# Source after starting the disposable app container as $name.
docker exec "$name" sh -c 'mkdir -p /repositories/ci-fixture && git init -q -b main /repositories/ci-fixture && printf "fixture\n" > /repositories/ci-fixture/README.md && git -C /repositories/ci-fixture add README.md && git -C /repositories/ci-fixture -c user.name=Fixture -c user.email=fixture@example.invalid commit -q -m fixture'
