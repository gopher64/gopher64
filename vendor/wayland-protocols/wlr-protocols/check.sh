#!/bin/sh -eux

for f in $(echo unstable/*.xml)
do
	wayland-scanner -s client-header "$f" /dev/null
	wayland-scanner -s server-header "$f" /dev/null
	wayland-scanner -s public-code "$f" /dev/null
	wayland-scanner -s private-code "$f" /dev/null
done
