#!/usr/bin/env bash
## Translate the given string into $langs using translate-shell, outputting to the yaml structure expected for locales/app.yml

langs="en lt es fr zh_CN zh_TW de"

printf "\"%s\":\n" "$@"
for lang in $langs; do
  result=$(trans -brief -no-auto -s en -t "${lang/_/-/}" "$@")
  printf "  %s: \"%s\"\n" "$lang" "$result"
done
