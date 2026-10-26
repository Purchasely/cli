#!/bin/sh
# Purchasely CLI managed installer (POSIX sh). Also used by `<name> update`.
# HOST below is set by pack-release.sh: the public host of the release files.
# Usage: install.sh [--version <ver>] [--keep <ver>] [--log <file>] [--root <dir>] [--upgrade-only]
# Exits: 0 installed or up to date, 1 download/checksum/smoke failure, 2 prerequisite missing, 3 lock held.
# The script never reads stdin (it can be the script itself) and never prints curl output: only fixed messages.
# The whole body is in main(), called on the last line: a truncated download runs nothing.
NAME=purchasely
HOST=

main() {
  set -eu
  ver="${PURCHASELY_VERSION:-}"
  keep=""
  log=""
  root_arg=""
  upgrade_only=0

  die() {
    code=$1
    shift
    printf '%s install: %s\n' "$NAME" "$*" >&2
    exit "$code"
  }

  while [ $# -gt 0 ]; do
    case "$1" in
      --version | --keep | --log | --root)
        [ $# -ge 2 ] || die 2 "missing value for $1"
        case "$1" in
          --version) ver=$2 ;;
          --keep) keep=$2 ;;
          --log) log=$2 ;;
          --root) root_arg=$2 ;;
        esac
        shift 2
        ;;
      --upgrade-only)
        upgrade_only=1
        shift
        ;;
      *) die 2 "unknown argument: $1" ;;
    esac
  done

  if [ -n "$log" ]; then
    exec >>"$log" 2>&1
  fi

  root=${root_arg:-${PURCHASELY_INSTALL_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/$NAME/install}}
  case "$root" in
    /*) ;;
    *) die 2 "the install root must be an absolute path: $root" ;;
  esac
  case "$root" in
    *"'"*) die 2 "the install root must not contain a single quote" ;;
  esac

  # --- prerequisites -----------------------------------------------------------
  if ! command -v node >/dev/null 2>&1 ||
    ! node -e 'process.exit(+process.versions.node.split(".")[0] >= 20 ? 0 : 1)' >/dev/null 2>&1; then
    die 2 "Node 20 or newer is required. Install it (for example 'brew install node'), then run this again."
  fi
  if ! command -v tar >/dev/null 2>&1 || ! command -v gzip >/dev/null 2>&1; then
    die 2 "tar and gzip are required. Install them, then run this again."
  fi
  [ -n "$HOST" ] || die 2 "this script has no host. Download it from the Purchasely CLI site."
  command -v curl >/dev/null 2>&1 || die 2 "curl is required. Install it, then run this again."

  # --- semver helpers (Node is a prerequisite) ---------------------------------
  # shortcut: pre-release tags compare as plain strings, upgrade if releases use real semver ordering.
  semver_js='const p=v=>{const[a,b]=v.split("-");return[a.split(".").map(n=>parseInt(n,10)||0),b]};
  const cmp=(u,w)=>{const[x,xb]=p(u),[y,yb]=p(w);
  for(let i=0;i<Math.max(x.length,y.length);i++){const d=(x[i]||0)-(y[i]||0);if(d)return d}
  if(xb===yb)return 0;if(!xb)return 1;if(!yb)return -1;return xb>yb?1:-1};'
  is_newer() { node -e "$semver_js"'process.exit(cmp(process.argv[1],process.argv[2])>0?0:1)' "$1" "$2"; }

  # --- curl calls: stderr goes to a private file and is never shown -----------
  errf=$(mktemp "${TMPDIR:-/tmp}/$NAME-install.XXXXXX")
  stage=""
  held=0
  lock="$root/staging/lock"
  cleanup() {
    rm -f "$errf" "$errf.latest"
    [ -z "$stage" ] || rm -rf "$stage"
    [ "$held" -eq 0 ] || rm -rf "$lock"
  }
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  fetch_fail() { die 1 "download failed from $HOST. Check your network, then run this again."; }
  # fetch <url> <file>: https only.
  fetch() { curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1" 2>"$errf" || fetch_fail; }

  fetch "$HOST/latest.json" "$errf.latest"
  latest=$(node -e 'process.stdout.write(String(JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).version))' "$errf.latest" 2>/dev/null) || fetch_fail
  latest=${latest#v}
  [ -n "$ver" ] || ver=$latest
  ver=${ver#v}
  case "$ver" in
    [0-9]*) ;;
    *) die 2 "invalid version: '$ver'" ;;
  esac
  case "$ver" in
    *[!0-9A-Za-z.-]*) die 2 "invalid version: '$ver'" ;;
  esac
  if [ "$ver" != "$latest" ]; then
    die 1 "only the latest version ($latest) is published on $HOST"
  fi

  # --- lock --------------------------------------------------------------------
  mkdir -p "$root/staging" "$root/releases" "$root/bin"
  if ! mkdir "$lock" 2>/dev/null; then
    pid=$(cat "$lock/pid" 2>/dev/null || true)
    if [ -z "$pid" ]; then
      sleep 1 # the holder may be between mkdir and writing its pid
      pid=$(cat "$lock/pid" 2>/dev/null || true)
    fi
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      die 3 "another install is running (pid $pid)"
    fi
    rm -rf "$lock"
    mkdir "$lock" 2>/dev/null || die 3 "another install holds the lock"
  fi
  held=1
  printf '%s\n' "$$" >"$lock/pid"
  for d in "$root"/staging/run-*; do
    [ ! -d "$d" ] || rm -rf "$d"
  done
  stage="$root/staging/run-$$"
  mkdir -p "$stage/pkg"

  # The man page of the release goes to the XDG man dir on every successful install (fresh, same version again,
  # update, --upgrade-only). A release without a page changes nothing. A failure is one warning, never an install failure.
  # `bindir` is set by link_bin and empty on --upgrade-only: the MANPATH hint is for an install that chose a bin dir.
  install_man() {
    src="$rel/man/$NAME.1"
    [ -f "$src" ] || return 0
    mandir=${XDG_DATA_HOME:-$HOME/.local/share}/man
    dst="$mandir/man1/$NAME.1"
    if ! { mkdir -p "$mandir/man1" && cp "$src" "$dst.tmp.$$" && mv -f "$dst.tmp.$$" "$dst"; } 2>/dev/null; then
      rm -f "$dst.tmp.$$" 2>/dev/null || true
      printf 'warning: could not install the man page in %s\n' "$mandir/man1"
      return 0
    fi
    [ -n "${bindir:-}" ] || return 0
    if command -v man >/dev/null 2>&1; then
      found=$(PATH="$bindir:$PATH" man -w "$NAME" 2>/dev/null || true)
      [ -n "$found" ] && cmp -s "$found" "$dst" && return 0
    else
      # shortcut: without man the rule of man is applied, only for the bin dir (man also reads the other PATH entries).
      # An empty MANPATH component (or MANPATH unset) adds the PATH-derived dirs, a MANPATH without one is the whole path.
      case ":${MANPATH-}:" in
        *":$mandir:"*) return 0 ;;
        *::*) [ "${bindir%/bin}/share/man" != "$mandir" ] || return 0 ;;
      esac
    fi
    # shellcheck disable=SC2016 # the $MANPATH in the hint is literal
    printf 'To read "man %s", add to your shell profile: export MANPATH="%s:$MANPATH"\n' "$NAME" "$mandir"
  }

  # --- decide what to do (under the lock) --------------------------------------
  prev=""
  if [ -f "$root/current-version" ]; then
    prev=$(cat "$root/current-version")
    case "$prev" in
      *[!0-9A-Za-z.-]*) prev="" ;;
    esac
  fi
  if [ "$upgrade_only" -eq 1 ] && [ -n "$prev" ] && ! is_newer "$ver" "$prev"; then
    rel="$root/releases/$prev"
    bindir=""
    install_man
    printf '%s %s is up to date\n' "$NAME" "$prev"
    exit 0
  fi

  rel="$root/releases/$ver"
  present=0
  if [ -f "$rel/dist/bin/$NAME.js" ]; then
    present=1
  elif [ -e "$rel" ]; then
    die 1 "$rel exists but is incomplete. Remove it, then run this again."
  fi

  write_launcher() {
    tmp="$root/bin/$NAME.tmp.$$"
    {
      printf '#!/bin/sh\n# Generated by install.sh. Do not edit.\nroot='"'"'%s'"'"'\nname=%s\n' "$root" "$NAME"
      cat <<'EOF'
v=$(cat "$root/current-version" 2>/dev/null) || { echo "$name: no installed version in $root" >&2; exit 1; }
case "$v" in
  ""|*[!0-9A-Za-z.-]*) echo "$name: invalid current-version in $root" >&2; exit 1 ;;
esac
PURCHASELY_MANAGED_INSTALL_ROOT=$root
export PURCHASELY_MANAGED_INSTALL_ROOT
exec node "$root/releases/$v/dist/bin/$name.js" "$@"
EOF
    } >"$tmp"
    chmod 755 "$tmp"
    mv -f "$tmp" "$root/bin/$NAME"
  }

  # The PATH symlink is made by a fresh install only: an update never moves it (the updater passes --root).
  # Bin dir: PURCHASELY_BIN_DIR, else a writable ~/.local/bin or ~/bin that is already on PATH, else
  # ~/.local/bin plus one guarded line in the profile of the login shell. Never sudo.
  link_bin() {
    [ "$upgrade_only" -eq 0 ] || return 0
    edit_profile=0
    if [ -n "${PURCHASELY_BIN_DIR:-}" ]; then
      bindir=$PURCHASELY_BIN_DIR
    else
      bindir=""
      for d in "$HOME/.local/bin" "$HOME/bin"; do
        if [ -d "$d" ] && [ -w "$d" ]; then
          case ":$PATH:" in
            *":$d:"*)
              bindir=$d
              break
              ;;
          esac
        fi
      done
      if [ -z "$bindir" ]; then
        bindir=$HOME/.local/bin
        edit_profile=1
      fi
    fi
    link="$bindir/$NAME"
    mkdir -p "$bindir"
    if [ -e "$link" ] && [ ! -L "$link" ]; then
      printf 'warning: %s exists and is not a symlink, left unchanged\n' "$link"
    else
      ln -sfn "$root/bin/$NAME" "$link"
    fi
    if [ "$edit_profile" -eq 1 ]; then
      marker='# added by the purchasely installer'
      case "${SHELL##*/}" in
        zsh) set -- "${ZDOTDIR:-$HOME}/.zshrc" ;;
        bash) set -- "$HOME/.bashrc" "$HOME/.bash_profile" ;;
        *) set -- "$HOME/.profile" ;;
      esac
      for f in "$@"; do
        grep -qF "$marker" "$f" 2>/dev/null ||
          printf '\n%s\n' "export PATH=\"\$HOME/.local/bin:\$PATH\" $marker" >>"$f"
      done
      printf 'open a new terminal if the command is not found\n'
    elif [ -n "${PURCHASELY_BIN_DIR:-}" ]; then
      # shellcheck disable=SC2016 # the $PATH in the hint is literal
      case ":$PATH:" in
        *":$bindir:"*) ;;
        *) printf 'Add %s to your PATH, for example in your shell profile:\n  export PATH="%s:$PATH"\n' "$bindir" "$bindir" ;;
      esac
    fi
    other=$(command -v "$NAME" 2>/dev/null || true)
    if [ -n "$other" ] && [ "$other" != "$link" ]; then
      printf 'warning: %s comes before %s on PATH (an old npm install?). Uninstall it to use the managed copy.\n' "$other" "$link"
    fi
  }

  if [ "$present" -eq 1 ] && [ "$prev" = "$ver" ]; then
    write_launcher
    link_bin
    install_man
    printf '%s %s is up to date\n' "$NAME" "$ver"
    exit 0
  fi

  # --- stage, verify, smoke ----------------------------------------------------
  if [ "$present" -eq 0 ]; then
    fetch "$HOST/$NAME-$ver.tgz" "$stage/$NAME-$ver.tgz"
    fetch "$HOST/SHA256SUMS" "$stage/SHA256SUMS"
    if command -v sha256sum >/dev/null 2>&1; then sum="sha256sum"; else sum="shasum -a 256"; fi
    line=$(awk -v f="$NAME-$ver.tgz" '$2==f' "$stage/SHA256SUMS" 2>/dev/null || true)
    if [ -z "$line" ] || ! (cd "$stage" && printf '%s\n' "$line" | $sum -c - >/dev/null 2>&1); then
      die 1 "checksum verification failed for $NAME-$ver.tgz. Nothing was installed."
    fi
    tar -xzf "$stage/$NAME-$ver.tgz" -C "$stage/pkg" --strip-components=1 ||
      die 1 "could not unpack $NAME-$ver.tgz. Nothing was installed."
    smoke="$stage/pkg"
  else
    smoke="$rel"
  fi
  out=$(PURCHASELY_DISABLE_AUTOUPDATE=1 node "$smoke/dist/bin/$NAME.js" --version 2>/dev/null || true)
  [ "$out" = "$ver" ] || die 1 "smoke test failed: expected version $ver. Nothing was switched."

  # --- switch ------------------------------------------------------------------
  # A release is never removed here (another session may run it); only the prune below deletes releases.
  [ "$present" -eq 1 ] || mv "$stage/pkg" "$rel"
  printf '%s\n' "$ver" >"$root/current-version.tmp.$$"
  mv -f "$root/current-version.tmp.$$" "$root/current-version"
  write_launcher
  link_bin
  install_man

  # --- prune: keep the 3 newest, the current one and --keep --------------------
  old=$(node -e "$semver_js"'
  const fs=require("fs");const [dir,cur,keep]=process.argv.slice(1);
  const all=fs.readdirSync(dir).sort((a,b)=>cmp(b,a));
  const kept=new Set([...all.slice(0,3),cur,keep]);
  console.log(all.filter(n=>!kept.has(n)).join("\n"))' "$root/releases" "$ver" "$keep")
  for r in $old; do
    rm -rf "$root/releases/$r"
  done

  printf '%s %s installed\n' "$NAME" "$ver"
}

main "$@"
