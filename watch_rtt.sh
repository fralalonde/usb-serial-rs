#! /bin/sh

# start OpenOCD RTT server and connect to it
# requires netcat (nc)

BASEDIR="$( cd "$( dirname "$0" )" && pwd )"
TELNET_PORT=4444
RTT_PORT=8745
# TODO support switching target debug<>release
ELF_FILE=$BASEDIR/target/thumbv6m-none-eabi/debug/usb-serial

if ! nc -z localhost $TELNET_PORT; then
  echo "OpenOCD not running? Else make sure it is listening for telnet on port $TELNET_PORT"
  # TODO start OpenOCD & flash $ELF_FILE ? assumed done by IDE (for debug) for now
  exit
else
  echo "OpenOCD running"
fi

if ! nc -z localhost $RTT_PORT; then
  block_addr=0x$(rust-nm -S $ELF_FILE | grep SEGGER_RTT | cut -d' ' -f1)
  echo "Starting RTT from block addr $block_addr"

  # see  https://stackoverflow.com/questions/48578664/capturing-telnet-timeout-from-bash
  nc localhost $TELNET_PORT <<EOF
rtt server start $RTT_PORT 0
rtt setup $block_addr 0x30 "SEGGER RTT"
rtt start
exit
EOF

  if ! nc -z localhost $RTT_PORT; then
    echo "RTT port still not up :("
    exit
  fi
else
  echo "RTT already up"
fi

# if using plain RTT https://crates.io/crates/rtt-target
#echo "Watching RTT"
#nc localhost $RTT_PORT

# if using defmt over RTT https://ferrous-systems.com/blog/gdb-and-defmt/
echo "Watching RTT/defmt"
nc localhost $RTT_PORT | defmt-print -e $ELF_FILE


