# Detached reopen helper: retry plugin pane open on ui_busy, log timing.
use strict; use Time::HiRes qw(time);
my ($w, $h, $gen) = @ARGV;
my $d = $0; $d =~ s{/[^/]*$}{};
open(my $log, '>>', "$d/spike.log") or die; select((select($log), $|=1)[0]);
my $ms = sub { int(time*1000) };
printf $log "%d helper gen=%s pid=%d sid=%s\n", $ms->(), $gen, $$, `ps -o sess= -p $$` =~ s/\s+//gr;
open(my $r, '>', "$d/ready"); close $r;
$SIG{HUP} = sub { printf $log "%d helper SIGHUP\n", $ms->(); exit 1 };
my $t0 = $ms->();
my $bin = $ENV{HERDR_BIN_PATH} || 'herdr';
for my $n (1..400) {
  my $out = `"$bin" plugin pane open --plugin "$ENV{HERDR_PLUGIN_ID}" --entrypoint "$ENV{HERDR_PLUGIN_ENTRYPOINT_ID}" --placement popup --width $w --height $h --env PIP_GEN=$gen --env PIP_W=$w --env PIP_H=$h --env PIP_AUTO=$ENV{PIP_AUTO} --env PIP_SOURCE=$ENV{PIP_SOURCE} 2>&1`;
  if ($? == 0) { printf $log "%d helper ok gen=%s attempts=%d helper_ms=%d\n", $ms->(), $gen, $n, $ms->()-$t0; exit 0 }
  if ($out !~ /ui_busy|already open/) { printf $log "%d helper FAIL attempt=%d: %s\n", $ms->(), $n, $out; exit 1 }
}
printf $log "%d helper GAVEUP\n", $ms->();
