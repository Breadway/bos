# BOS default zsh config — Powerlevel10k prompt + plugins + pywal palette.
#
# Mirrors the BOS dev shell, but sources plugins from the distro packages
# (/usr/share/zsh/...) instead of oh-my-zsh, so there's no framework to manage.
# Customise the prompt with `p10k configure` (rewrites ~/.p10k.zsh).

# Enable Powerlevel10k instant prompt. Should stay close to the top of ~/.zshrc.
# Initialization code that may require console input (password prompts, [y/n]
# confirmations, etc.) must go above this block; everything else may go below.
if [[ -r "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh" ]]; then
  source "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh"
fi

# History
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt HIST_IGNORE_DUPS HIST_IGNORE_SPACE SHARE_HISTORY

# Completion
autoload -Uz compinit && compinit
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Z}'

# Emacs-style key bindings
bindkey -e

# Prompt — Powerlevel10k (republished to [breadway] as zsh-theme-powerlevel10k).
source /usr/share/zsh-theme-powerlevel10k/powerlevel10k.zsh-theme

# Plugins (order matters: syntax-highlighting must be sourced LAST).
ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE='fg=60'
source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh 2>/dev/null
source /usr/share/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh 2>/dev/null
source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh 2>/dev/null

# history-substring-search: ↑/↓ search history by the typed prefix.
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down

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

# Updates — bos-update runs both channels (pacman + bakery). pacman aliased to
# sudo so `pacman -Syu` etc. just work.
alias update='bos-update'
alias pacman='sudo pacman'

# ~/.local/bin holds the bread* binaries baked in at build time.
export PATH="$HOME/.local/bin:$PATH"

# Powerlevel10k prompt configuration.
[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh

# Import pywal colour palette (drives the terminal colours from the wallpaper).
if [ -f "$HOME/.cache/wal/sequences" ]; then
    cat "$HOME/.cache/wal/sequences"
fi
