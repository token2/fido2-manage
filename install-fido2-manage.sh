#!/bin/bash
git clone https://github.com/Token2/fido2-manage.git
cd fido2-manage
sudo apt install -y zlib1g-dev pkg-config cmake libcbor-dev libpcsclite-dev libssl-dev libudev-dev
rm -rf build && mkdir build && cd build && cmake -USE_PCSC=ON ..
cd ..
make -C build
sudo make -C build install
sudo ldconfig
chmod 755 fido2-manage.sh
sudo apt-get -y install python3-tk
