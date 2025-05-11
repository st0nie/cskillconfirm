#!/bin/bash

git clone https://github.com/alsa-project/alsa-lib.git --depth 1
cd alsa-lib
apk add --update --no-cache \
    autoconf \
    automake \
    libtool \
    make \
    gcc \
    g++ \
    linux-headers \
    pkgconf \
    alsa-lib-dev

libtoolize --force --copy --automake
aclocal
autoheader
automake --foreign --copy --add-missing
autoconf
./configure --enable-shared=no --enable-static=yes
make -j$(nproc) install