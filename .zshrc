HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
autoload -Uz compinit
compinit
source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

WORDCHARS=''

# Стандартный режим перестроения клавиш (emacs-style)
bindkey -e

# Базовые клавиши удаления и перемещения
bindkey '^[[3~'  delete-char                      # Delete
bindkey '^H'     backward-kill-word               # Ctrl + Backspace
bindkey '^[[3;5~' kill-word                        # Ctrl + Delete

# Перемещение по словам (Ctrl + Стрелки)
bindkey '^[[1;5C' forward-word                    # Ctrl + Right
bindkey '^[[1;5D' backward-word                   # Ctrl + Left

# Home и End
bindkey '^[[H'    beginning-of-line                # Home
bindkey '^[[F'    end-of-line                      # End

PROMPT='%F{183}%n%f@%F{white}%m%f:%F{blue}%~%f %# '
export PATH="$HOME/.dev/flutter_sdk/flutter/bin:$PATH"

alias startniri="dbus-run-session niri --session"
alias vcc="~/convert.sh"

alias comfy='cd "$HOME/comfy" && source venv/bin/activate && cd "$HOME/comfy/ComfyUI" && python main.py --gpu-only'


. "$HOME/.local/bin/env"
