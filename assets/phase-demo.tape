Output assets/phase-demo.gif
Output assets/phase-demo.mp4

Require ./target/release/phase

Set Shell "zsh"
Set FontSize 18
Set Width 1100
Set Height 680
Set Padding 20
Set Framerate 12
Set Theme "tokyonight"
Set WindowBar ColorfulRight
Set WindowBarSize 36

Hide
Type "env -u NO_COLOR COLORTERM=truecolor TERM=xterm-256color ./target/release/phase --demo --no-audio"
Enter
Sleep 1500ms
Show

Sleep 1800ms
Type "n"
Sleep 1200ms
Tab
Sleep 1300ms
Tab
Sleep 4500ms
Tab
Sleep 1200ms
Tab
Sleep 1200ms
Type "?"
Sleep 1600ms
Type "?"
Sleep 400ms
Type "q"
