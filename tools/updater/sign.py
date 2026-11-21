"""Sign a release for the Constellations updater and write latest.json.

    python tools/updater/sign.py --version 0.2.0 --notes "What is new" \\
        --windows dist/Constellations-Setup-0.2.0.exe \\
        --windows-url https://github.com/13LA7E/Odys-Brows/releases/download/v0.2.0/Constellations-Setup-0.2.0.exe \\
        --out releases/latest.json

The private key is NOT in the repository. It lives in your own profile folder
(default: ~/.constellations/update-signing-key.pem, made once with --make-key). Keep a copy somewhere
safe: if it is lost, no installed Constellations can be updated any more; if it leaks, anyone can
send updates. The matching public key is built into the program (app/shell/src/updater.rs).

What is signed for each file: "constellations-update/v1\\n<version>\\n<platform>\\n<sha256>\\n<size>".
"""
import argparse, base64, hashlib, json, os, sys, datetime

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

DEFAULT_KEY = os.path.expanduser('~/.constellations/update-signing-key.pem')


def load_key(path):
    with open(path, 'rb') as f:
        return serialization.load_pem_private_key(f.read(), None)


def make_key(path):
    if os.path.exists(path):
        raise SystemExit('A key already exists at %s (not overwritten).' % path)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    key = Ed25519PrivateKey.generate()
    with open(path, 'wb') as f:
        f.write(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))
    return key


def public_b64(key):
    return base64.b64encode(key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)).decode()


def signed_message(version, platform, sha256, size):
    return 'constellations-update/v1\n%s\n%s\n%s\n%d' % (version, platform, sha256, size)


def entry(key, version, platform, file_path, url):
    data = open(file_path, 'rb').read()
    sha = hashlib.sha256(data).hexdigest()
    sig = base64.b64encode(key.sign(signed_message(version, platform, sha, len(data)).encode())).decode()
    return {'url': url, 'sha256': sha, 'size': len(data), 'signature': sig}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--key', default=DEFAULT_KEY)
    ap.add_argument('--make-key', action='store_true', help='create the signing key and print its public half')
    ap.add_argument('--version')
    ap.add_argument('--notes', default='')
    ap.add_argument('--windows'); ap.add_argument('--windows-url')
    ap.add_argument('--linux'); ap.add_argument('--linux-url')
    ap.add_argument('--out', default='latest.json')
    a = ap.parse_args()
    if a.make_key:
        print('public key:', public_b64(make_key(a.key)))
        return
    key = load_key(a.key)
    if not a.version:
        raise SystemExit('--version is required')
    platforms = {}
    if a.windows:
        platforms['windows-x86_64'] = entry(key, a.version, 'windows-x86_64', a.windows, a.windows_url)
    if a.linux:
        platforms['linux-x86_64'] = entry(key, a.version, 'linux-x86_64', a.linux, a.linux_url)
    if not platforms:
        raise SystemExit('give at least one of --windows or --linux')
    manifest = {'version': a.version, 'notes': a.notes, 'pub_date': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'), 'platforms': platforms}
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    with open(a.out, 'w', encoding='utf8') as f:
        json.dump(manifest, f, indent=2)
    print('wrote', a.out, '(public key in the program:', public_b64(key) + ')')


if __name__ == '__main__':
    main()
