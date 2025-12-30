#!/bin/sh

set -x

mkdir -p ~/.config/systemd/user/
cp ~/calendar-display/calendar-display.service ~/calendar-display/calendar-display-usr1.service ~/calendar-display/calendar-display-usr1.timer ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now calendar-display.service
systemctl --user enable --now calendar-display-usr1.timer
