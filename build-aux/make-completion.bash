#!/usr/bin/env bash
#
# Shell tab-completion for the Swatchbook Makefile.
# Works with both bash and zsh (via bashcompinit).
#
# Usage — add ONE of these lines to your ~/.bashrc or ~/.zshrc:
#
#   source /path/to/swatchbook/build-aux/make-completion.bash
#
# Or, from inside the project directory:
#
#   source build-aux/make-completion.bash

# ── Target list ──────────────────────────────────────────────────────────────
# Extracted from the Makefile's ## comments so this stays in sync automatically.
_swatchbook_make_targets() {
    local makefile
    makefile="$(git -C "$(pwd)" rev-parse --show-toplevel 2>/dev/null)/Makefile"
    [ -f "$makefile" ] || makefile="$(pwd)/Makefile"
    [ -f "$makefile" ] || return

    grep -E '^[a-zA-Z_-]+:.*##' "$makefile" | awk -F':.*##' '{print $1}' | tr -d ' '
}

# ── Bash completion ───────────────────────────────────────────────────────────
_swatchbook_make_complete_bash() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local targets
    targets="$(_swatchbook_make_targets)"
    # shellcheck disable=SC2207
    COMPREPLY=($(compgen -W "$targets" -- "$cur"))
}

# Register for bash
if [ -n "$BASH_VERSION" ]; then
    complete -F _swatchbook_make_complete_bash make
fi

# ── Zsh completion ────────────────────────────────────────────────────────────
_swatchbook_make_complete_zsh() {
    local targets
    targets="$(_swatchbook_make_targets)"

    # Build an associative array of target→description from the Makefile
    local makefile
    makefile="$(git -C "$(pwd)" rev-parse --show-toplevel 2>/dev/null)/Makefile"
    [ -f "$makefile" ] || makefile="$(pwd)/Makefile"

    local -a completions
    if [ -f "$makefile" ]; then
        while IFS= read -r line; do
            local target desc
            target=$(echo "$line" | awk -F':.*##' '{print $1}' | tr -d ' ')
            desc=$(echo "$line" | awk -F'##' '{print $2}' | sed 's/^ *//')
            completions+=("${target}:${desc}")
        done < <(grep -E '^[a-zA-Z_-]+:.*##' "$makefile")
    fi

    _describe 'make target' completions
}

# Register for zsh
if [ -n "$ZSH_VERSION" ]; then
    autoload -Uz compinit 2>/dev/null
    compinit -C 2>/dev/null
    compdef _swatchbook_make_complete_zsh make
fi
