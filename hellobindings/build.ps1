
get-process | where-object {$_.MainWindowTitle -eq "Helloworld (DEBUG)"} | stop-process
cargo build
