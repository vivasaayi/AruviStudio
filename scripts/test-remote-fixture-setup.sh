#!/usr/bin/env bash
# Sourced with the disposable fixture container name in $name.
docker exec "$name" sh -c 'for d in /repositories/workspace-choice-a /repositories/workspace-choice-b; do mkdir -p "$d" && git init -q -b main "$d" || exit 1; done'
docker exec "$name" sh -c 'd=/repositories/code-editor-fixture; mkdir -p "$d" && git init -q -b main "$d" && printf "const first = 1;\n" > "$d/first.js" && printf "const second = 2;\n" > "$d/second.js" && printf "one\r\ntwo\n" > "$d/mixed.js" && printf "one\r\ntwo\r\n" > "$d/windows.js"'
docker exec "$name" sh -c '
  d=/repositories/remote-fixture
  mkdir -p "$d" && git init -q "$d" && printf "base line\n" > "$d/git-diff.txt" && git -C "$d" add git-diff.txt &&
  git -C "$d" -c user.name=Fixture -c user.email=fixture@example.invalid commit -q -m fixture &&
  printf "staged line\n" > "$d/git-diff.txt" && git -C "$d" add git-diff.txt && printf "working line\n" > "$d/git-diff.txt" &&
  mkdir "$d/second" "$d/node_modules" "$d/nested" && git init -q "$d/second" && printf "nested fixture\n" > "$d/nested/target-file.txt" && : > "$d/patch-safety-fixture.txt" && ln "$d/patch-safety-fixture.txt" "$d/patch-alias.txt" &&
  printf "workspace inspection fixture\n" > "$d/inspection.txt" && printf "windows line\r\n" > "$d/windows.txt" &&
  printf "one\r\ntwo\n" > "$d/mixed.txt" && printf "hidden fixture\n" > "$d/node_modules/excluded.txt" &&
  ln -s /etc/passwd "$d/outside-link" && truncate -s 2097153 "$d/oversized.txt" && yes é | head -c 3000 > "$d/aaa_unicode.txt"
'
