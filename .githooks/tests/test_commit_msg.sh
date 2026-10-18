#!/bin/sh
# Testes do hook commit-msg.

set -u

HOOK="$(dirname "$0")/../commit-msg"
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

failures=0

check() {
  expected=$1
  message=$2
  printf '%s\n' "$message" >"$TMP"
  if sh "$HOOK" "$TMP" 2>/dev/null; then result=accept; else result=reject; fi
  if [ "$result" != "$expected" ]; then
    echo "FALHOU: esperado $expected, obtido $result para: $message"
    failures=$((failures + 1))
  fi
}

check accept "feat: adicionar fila de jobs"
check accept "fix(queue): retomar fila após reinício"
check accept "chore(deps)!: atualizar tauri para 3.0"
check accept "docs(planejamento/fluxos): detalhar templates"
check accept "refactor(sd-cpp): extrair adaptador"
check accept "Merge branch 'main' into feat/fila"
check accept "Revert \"feat: adicionar fila de jobs\""
check accept "fixup! feat: adicionar fila de jobs"
check accept "$(printf '# comentário do editor\n\nfeat: ignora comentários')"
check accept "$(printf 'feat: corpo é ignorado\n\nqualquer coisa aqui')"

check reject "Adicionar fila de jobs"
check reject "feature: adicionar fila"
check reject "feat adicionar fila"
check reject "feat:adicionar fila"
check reject "feat:  espaço duplo"
check reject "feat(Queue): escopo com maiúscula"
check reject "feat(): escopo vazio"
check reject "FEAT: tipo em maiúscula"
check reject ""
check reject "feat: $(printf '%0100d' 0)"

if [ "$failures" -gt 0 ]; then
  echo "$failures teste(s) do commit-msg falharam."
  exit 1
fi
echo "Testes do commit-msg: ok"
