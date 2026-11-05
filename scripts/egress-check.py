"""Prove a reachable TCP endpoint is denied inside the macOS sandbox."""
import errno
import json
import socket
import sys

def denied(target, socket_factory=socket.socket):
    try:
        with socket_factory(target['family'], target['kind'], target['protocol']) as sock:
            sock.settimeout(5)
            sock.connect(tuple(target['address']))
    except OSError as exc:
        if exc.errno in {errno.EACCES, errno.EPERM}:
            return True
        raise
    return False


def main():
    mode, path = sys.argv[1:]
    if mode == 'probe':
        for family, kind, protocol, _, address in socket.getaddrinfo('github.com', 443, type=socket.SOCK_STREAM):
            try:
                with socket.socket(family, kind, protocol) as sock:
                    sock.settimeout(5)
                    sock.connect(address)
                with open(path, 'x', encoding='utf-8') as out:
                    json.dump({'family': family, 'kind': kind, 'protocol': protocol, 'address': address}, out)
                print('External TCP connection succeeded before sandboxing.')
                break
            except OSError:
                continue
        else:
            raise SystemExit('Cannot establish pre-sandbox reachability; failing closed')
    elif mode == 'deny':
        with open(path, encoding='utf-8') as inp:
            target = json.load(inp)
        try:
            if not denied(target):
                raise SystemExit('Sandbox allowed outbound TCP; failing closed')
        except OSError as exc:
            raise SystemExit(f'Expected sandbox permission denial, received errno {exc.errno}') from exc
        print('Sandbox denied the previously reachable TCP connection.')
    else:
        raise SystemExit('Invalid mode')


if __name__ == '__main__':
    main()
