#!/bin/sh
# Runs inside the VM. Checks that a display manager starts the installed "Patina" session the way
# it would for a user picking it at the login screen: through logind, on seat0, with no seatd.
# Installs the display manager if needed, logs in user dev automatically into the Patina
# session, reports what is running, then turns the display manager off again.
#
# Usage: dm-test.sh sddm|gdm3|lightdm [deb]   deb defaults to ~/PatineDE/dist/bookworm/*.deb
set -eu
DM=${1:?sddm, gdm3 or lightdm}
DEB=${2:-$(ls "$HOME"/PatineDE/dist/bookworm/patina-desktop_*.deb)}
USER_NAME=$(id -un)

sudo apt-get install -y -q "$DEB" >/dev/null
# seatd from earlier tests would hold the seat; logind must have it now.
sudo systemctl stop seatd 2>/dev/null || true
pkill -u "$USER_NAME" -f '[p]atina-' 2>/dev/null || true
rm -rf "$HOME/.local/state/patina"

case $DM in
  sddm)
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends sddm >/dev/null
    sudo mkdir -p /etc/sddm.conf.d
    printf '[Autologin]\nUser=%s\nSession=patina.desktop\n' "$USER_NAME" |
      sudo tee /etc/sddm.conf.d/zz-patina-test.conf >/dev/null
    ;;
  gdm3)
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends gdm3 >/dev/null
    sudo mkdir -p /etc/gdm3 /var/lib/AccountsService/users
    printf '[daemon]\nAutomaticLoginEnable=true\nAutomaticLogin=%s\n' "$USER_NAME" |
      sudo tee /etc/gdm3/daemon.conf >/dev/null
    # GDM takes the session from AccountsService.
    printf '[User]\nSession=patina\nSystemAccount=false\n' |
      sudo tee "/var/lib/AccountsService/users/$USER_NAME" >/dev/null
    ;;
  lightdm)
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends \
      lightdm lightdm-gtk-greeter >/dev/null
    sudo mkdir -p /etc/lightdm/lightdm.conf.d
    printf '[Seat:*]\nautologin-user=%s\nautologin-session=patina\n' "$USER_NAME" |
      sudo tee /etc/lightdm/lightdm.conf.d/zz-patina-test.conf >/dev/null
    ;;
  *) echo "unknown display manager: $DM" >&2; exit 2 ;;
esac

sudo systemctl restart "$DM"
sleep 25

echo "--- logind sessions"
loginctl list-sessions --no-legend | while read -r id _ user seat _; do
  [ "$user" = "$USER_NAME" ] || continue
  loginctl show-session "$id" -p Type -p Class -p Seat -p Desktop -p Service -p State -p Active
  echo
done
echo "--- processes"
pgrep -af -u "$USER_NAME" 'patina|dbus-run' | cut -c1-100
echo "--- session.log"
sed 's/\x1b\[[0-9;]*m//g' "$HOME/.local/state/patina/session.log" 2>/dev/null |
  grep -E "session started|libseat|logind|seat|GPU|display connected|started for|ERROR|panic" |
  cut -c1-160 | head -15

if [ "${KEEP:-0}" != 1 ]; then
  sudo systemctl stop "$DM"
  sudo systemctl disable "$DM" >/dev/null 2>&1 || true
  sudo rm -f /etc/sddm.conf.d/zz-patina-test.conf /etc/lightdm/lightdm.conf.d/zz-patina-test.conf
  echo "--- $DM stopped"
fi
