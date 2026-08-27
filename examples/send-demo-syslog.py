#!/usr/bin/env python3
"""Sends a synthetic network log stream to a LogLens syslog listener.

Everything here is invented. Addresses come from the ranges RFC 5737 and
RFC 3849 reserve for documentation, and the MAC addresses use vendor prefixes
with a zeroed device half, so nothing in this file corresponds to real
equipment.

Usage:
    python3 send-demo-syslog.py [host:port] [--rate N] [--once]
"""

from __future__ import annotations

import argparse
import random
import socket
import sys
import time
from datetime import datetime, timedelta

# Vendor prefixes with an invented device half.
CLIENTS = [
    "74:ac:b9:00:00:11",  # Ubiquiti
    "3c:22:fb:00:00:22",  # Apple
    "b8:27:eb:00:00:33",  # Raspberry Pi
    "a2:00:00:00:00:44",  # locally administered, a randomized phone MAC
]
ACCESS_POINTS = ["ap-lobby", "ap-office", "ap-lab"]

# A day in the life of a small site: mostly ordinary, with one client that
# keeps failing on a stale pre-shared key.
def unifi_lines() -> list[str]:
    client = random.choice(CLIENTS)
    ap = random.choice(ACCESS_POINTS)
    radio = random.choice(["ath0", "ath1"])
    return [
        f"{ap} hostapd: {radio}: STA {client} IEEE 802.11: associated",
        f"{ap} hostapd: {radio}: STA {client} WPA: pairwise key handshake completed",
        f"{ap} hostapd: {radio}: STA {client} IEEE 802.11: disassociated",
        f"gw dnsmasq-dhcp[1123]: DHCPACK(br0) 192.0.2.{random.randint(20, 200)} {client} demo-device",
    ]


def failing_client_lines() -> list[str]:
    """The one recurring problem the demo is built around."""
    client = CLIENTS[3]
    return [
        f"ap-lab hostapd: ath0: STA {client} WPA: invalid MIC in msg 2/4 of 4-Way Handshake",
        f"ap-lab hostapd: ath0: STA {client} IEEE 802.11: deauthenticated due to local deauth request",
    ]


def firewall_lines() -> list[str]:
    src = f"203.0.113.{random.randint(2, 250)}"
    dst = f"192.0.2.{random.randint(2, 250)}"
    port = random.choice([22, 23, 445, 3389, 8080])
    return [
        f"gw kernel: [WAN_IN-2000-D]IN=eth0 OUT=br0 SRC={src} DST={dst} LEN=60 PROTO=TCP SPT={random.randint(1024, 65000)} DPT={port}",
    ]


def pfsense_line() -> str:
    src = f"198.51.100.{random.randint(2, 250)}"
    dst = f"192.0.2.{random.randint(2, 250)}"
    return (
        "fw filterlog: 5,,,1000000103,igb0,match,block,in,4,0x0,,64,"
        f"{random.randint(1, 60000)},0,DF,6,tcp,60,{src},{dst},"
        f"{random.randint(1024, 65000)},443,0,S,1,0,64240,,mss"
    )


def mikrotik_line() -> str:
    src = f"192.0.2.{random.randint(2, 250)}"
    return (
        f"firewall,info drop: in:ether1 out:ether2, src-mac {random.choice(CLIENTS)}, "
        f"proto UDP, {src}:{random.randint(1024, 65000)}->198.51.100.9:5353, len 80"
    )


def frame(body: str, severity: int, when: datetime) -> bytes:
    """Wraps a message in an RFC 3164 header, which is what appliances send."""
    priority = 16 * 8 + severity  # local0
    stamp = when.strftime("%b %e %H:%M:%S")
    return f"<{priority}>{stamp} {body}".encode()


def build_batch(when: datetime) -> list[bytes]:
    batch = [frame(line, 6, when) for line in unifi_lines()]
    batch += [frame(line, 4, when) for line in failing_client_lines()]
    batch += [frame(line, 4, when) for line in firewall_lines()]
    batch.append(frame(pfsense_line(), 4, when))
    batch.append(frame(mikrotik_line(), 4, when))
    random.shuffle(batch)
    return batch


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", nargs="?", default="127.0.0.1:5514")
    parser.add_argument("--rate", type=float, default=2.0, help="batches per second")
    parser.add_argument("--once", action="store_true", help="send one batch and exit")
    parser.add_argument("--backfill", type=int, default=0, help="minutes of history to send first")
    args = parser.parse_args()

    host, _, port = args.target.partition(":")
    address = (host, int(port or 5514))
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    sent = 0
    if args.backfill:
        start = datetime.now() - timedelta(minutes=args.backfill)
        for minute in range(args.backfill):
            for datagram in build_batch(start + timedelta(minutes=minute)):
                sock.sendto(datagram, address)
                sent += 1
            # Without a pause the receive buffer overflows and the kernel
            # drops the rest of the burst. UDP syslog has no retransmit, so
            # what is dropped here is simply gone.
            time.sleep(0.02)
        print(f"backfilled {sent} lines over {args.backfill} minutes", file=sys.stderr)

    if args.once:
        for datagram in build_batch(datetime.now()):
            sock.sendto(datagram, address)
            sent += 1
        print(f"sent {sent} lines to {host}:{address[1]}", file=sys.stderr)
        return 0

    print(f"sending to {host}:{address[1]}, stop with Ctrl+C", file=sys.stderr)
    try:
        while True:
            for datagram in build_batch(datetime.now()):
                sock.sendto(datagram, address)
                sent += 1
            time.sleep(1.0 / args.rate)
    except KeyboardInterrupt:
        print(f"\nstopped after {sent} lines", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
