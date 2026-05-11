#!/bin/bash

echo tmtcopen by İsmail Uygar
echo Kurulum Başlatılıyor...

echo maim indiriliyor
sudo apt install maim -y

echo tmtcopen binarysi kopyalanıyor...
sudo cp tmtcopen /usr/bin/tmtcopen
sudo chmod +x /usr/bin/tmtcopen

echo tmtcopen ikonu kopyalanıyor...
sudo cp logo.png /usr/share/pixmaps/tmtcopen.png

echo .desktop dosyası kopyalanıyor...
sudo cp tmtcopen.desktop /usr/share/applications/tmtcopen.desktop
sudo chmod +x /usr/share/applications/tmtcopen.desktop