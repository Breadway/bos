# BOS default zsh config — quality-of-life defaults, easy to extend.

# History
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt HIST_IGNORE_DUPS HIST_IGNORE_SPACE SHARE_HISTORY

# Completion
autoload -Uz compinit && compinit
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Z}'

# Key bindings (emacs style + common extras)
bindkey -e
bindkey '^[[A' history-search-backward
bindkey '^[[B' history-search-forward

# fzf — fuzzy history search on Ctrl+R, fuzzy file find on Ctrl+T
if command -v fzf &>/dev/null; then
    source /usr/share/fzf/key-bindings.zsh 2>/dev/null || true
    source /usr/share/fzf/completion.zsh 2>/dev/null || true
    export FZF_DEFAULT_OPTS='--height 40% --layout=reverse --border'
fi

# zoxide — smarter cd (type z instead of cd)
if command -v zoxide &>/dev/null; then
    eval "$(zoxide init zsh)"
fi

# Modern replacements with fallbacks
if command -v eza &>/dev/null; then
    alias ls='eza --icons --group-directories-first'
    alias ll='eza -la --icons --group-directories-first --git'
    alias lt='eza --tree --icons --level=2'
else
    alias ls='ls --color=auto'
    alias ll='ls -la'
fi

if command -v bat &>/dev/null; then
    alias cat='bat --style=plain --paging=never'
fi

# General aliases
alias clr='clear'
alias ..='cd ..'
alias ...='cd ../..'
alias mkdir='mkdir -p'
alias cp='cp -i'
alias mv='mv -i'
alias df='df -h'
alias free='free -h'
alias grep='grep --color=auto'
alias ip='ip --color=auto'

# bakery / bread
alias update='bakery update'

# Prompt — simple and fast (no starship dep)
autoload -Uz vcs_info
precmd() { vcs_info }
zstyle ':vcs_info:git:*' formats ' (%b)'
setopt PROMPT_SUBST
PROMPT='%F{cyan}%~%f%F{yellow}${vcs_info_msg_0_}%f %(?.%F{green}.%F{red})❯%f '
