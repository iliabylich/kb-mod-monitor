default:
    @just --list

setup build='debug':
    meson setup builddir --buildtype={{build}}

build:
    meson compile -C builddir

run:
    builddir/caps-lock-daemon

clean:
    rm -rf builddir

test-install:
    @just clean
    rm -rf test-install
    meson setup builddir --buildtype=release --prefix=$PWD/test-install/usr
    meson compile -C builddir
    meson install -C builddir
    tree -C test-install
