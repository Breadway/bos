#!/usr/bin/env bash
# Configure the installed initramfs for encrypted root before Calamares runs
# initcpio. luksbootkeyfile creates and registers this key while the user's
# passphrase is still available to Calamares.
set -uo pipefail

ROOT_SRC="$(findmnt -no SOURCE / | sed 's/\[.*\]//')"
if [[ "$(lsblk -no TYPE "$ROOT_SRC" 2>/dev/null)" != "crypt" ]]; then
    exit 0
fi

ROOT_MAPPER="$(basename -- "$ROOT_SRC")"
LUKS_PARENT="$(lsblk -no PKNAME "$ROOT_SRC" 2>/dev/null | head -n1)"
if [[ -n "$LUKS_PARENT" ]]; then
    LUKS_DEVICE="/dev/$LUKS_PARENT"
else
    LUKS_DEVICE="$(cryptsetup status "$ROOT_MAPPER" 2>/dev/null \
        | awk '$1 == "device:" { print $2; exit }')"
fi
if [[ -z "$LUKS_DEVICE" || ! -b "$LUKS_DEVICE" ]]; then
    echo "WARN: could not determine the LUKS device for encrypted root"
    exit 0
fi

LUKS_UUID="$(cryptsetup luksUUID "$LUKS_DEVICE" 2>/dev/null)"
if [[ -z "$LUKS_UUID" ]]; then
    echo "WARN: could not read the LUKS UUID for $LUKS_DEVICE"
    exit 0
fi

if [[ ! -f /crypto_keyfile.bin ]]; then
    echo "WARN: Calamares did not create /crypto_keyfile.bin; initramfs will prompt for a passphrase"
fi

if [[ -f /crypto_keyfile.bin ]]; then
    chmod 0600 /crypto_keyfile.bin \
        || echo "WARN: could not restrict permissions on /crypto_keyfile.bin"
    if [[ -f /etc/mkinitcpio.conf ]] \
       && ! grep -qF 'FILES+=(/crypto_keyfile.bin)' /etc/mkinitcpio.conf; then
        printf '\n# Calamares LUKS boot key, read after GRUB unlocks /boot.\nFILES+=(/crypto_keyfile.bin)\n' \
            >> /etc/mkinitcpio.conf \
            || echo "WARN: adding the LUKS keyfile to mkinitcpio failed"
    fi
fi

ENCRYPT_HOOK=""
if [[ -f /etc/mkinitcpio.conf ]]; then
    if grep -qE '^HOOKS=.*\bsystemd\b' /etc/mkinitcpio.conf; then
        ENCRYPT_HOOK="sd-encrypt"
    else
        ENCRYPT_HOOK="encrypt"
    fi
    if ! grep -qE "^HOOKS=.*[[:space:]]${ENCRYPT_HOOK}([[:space:]]|\\))" /etc/mkinitcpio.conf; then
        sed -i "s/^\(HOOKS=.*\\bblock\\b\)/\1 ${ENCRYPT_HOOK}/" /etc/mkinitcpio.conf \
            || echo "WARN: adding ${ENCRYPT_HOOK} to mkinitcpio failed"
    fi
fi

# fstab runs after luksbootkeyfile and writes the mapper, UUID, key path and
# x-initrd.attach option to /etc/crypttab. The sd-encrypt hook copies that
# entry into the initramfs so systemd can unlock root before mounting it.
if [[ "$ENCRYPT_HOOK" == "sd-encrypt" ]] \
   && ! grep -qE "^${ROOT_MAPPER}[[:space:]]+UUID=${LUKS_UUID}[[:space:]]+/crypto_keyfile\\.bin[[:space:]].*x-initrd\\.attach" /etc/crypttab 2>/dev/null; then
    echo "WARN: /etc/crypttab has no initramfs entry for $ROOT_MAPPER"
fi
