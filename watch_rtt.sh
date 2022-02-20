#! /bin/sh

# start OpenOCD RTT server and connect to it
# requires netcat (nc)

BASEDIR="$( cd "$( dirname "$0" )" && pwd )"
TELNET_PORT=4444
RTT_PORT=8745
ELF_FILE=$BASEDIR/target/thumbv6m-none-eabi/debug/usb-serial

if ! nc -z localhost $TELNET_PORT; then
  echo "OpenOCD not running? Else make sure it is listening for telnet on port $TELNET_PORT"
  exit
else
  echo "OpenOCD running"
fi

if ! nc -z localhost $RTT_PORT; then
  block_addr=0x$(rust-nm -S $ELF_FILE | grep SEGGER_RTT | cut -d' ' -f1)
  echo "Starting RTT from block addr $block_addr"

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



#echo "Watching RTT/defmt"
#nc localhost $RTT_PORT | defmt-print -e $ELF_FILE

sleep 2
echo "Watching RTT"
nc localhost $RTT_PORT
