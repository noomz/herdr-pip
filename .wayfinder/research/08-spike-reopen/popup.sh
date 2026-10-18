#!/bin/sh
# Popup: show size; '+'/'-' (or PIP_AUTO countdown) -> spawn detached helper, exit.
D=$(cd "$(dirname "$0")" && pwd)
LOG="$D/spike.log"
now() { perl -MTime::HiRes=time -e 'printf "%d", time*1000'; }
GEN=${PIP_GEN:-0}; W=${PIP_W:-40}; H=${PIP_H:-10}
echo "$(now) start gen=$GEN outer=${W}x${H} pty=$(stty size) pid=$$ sid=$(ps -o sess= -p $$ 2>/dev/null)" >> "$LOG"
printf 'gen %s  outer %sx%s  pty %s\n+/- resize, q quit\n' "$GEN" "$W" "$H" "$(stty size)"
reopen() {
  rm -f "$D/ready"
  if [ -n "$PIP_NOSETSID" ]; then
    perl "$D/helper.pl" "$1" "$2" $((GEN+1)) &
  else
    perl -MPOSIX -e 'POSIX::setsid() or die; exec @ARGV' perl "$D/helper.pl" "$1" "$2" $((GEN+1)) &
  fi
  i=0; while [ ! -e "$D/ready" ] && [ $i -lt 200 ]; do i=$((i+1)); sleep 0.005; done
  echo "$(now) exit gen=$GEN" >> "$LOG"
  exit 0
}
if [ -n "$PIP_AUTO" ] && [ "$GEN" -lt "$PIP_AUTO" ]; then
  sleep 0.6
  if [ $((GEN % 2)) -eq 0 ]; then reopen $((W+20)) $((H+6)); else reopen $((W-20)) $((H-6)); fi
fi
stty raw -echo
while :; do
  k=$(dd bs=1 count=1 2>/dev/null)
  case "$k" in
    +) stty sane; reopen $((W+10)) $((H+3));;
    -) stty sane; reopen $((W-10)) $((H-3));;
    q) stty sane; echo "$(now) quit gen=$GEN" >> "$LOG"; exit 0;;
  esac
done
