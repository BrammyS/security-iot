#!/bin/bash

echo gpio | sudo tee /sys/class/leds/led0/trigger
sudo chown root:gpio /sys/class/leds/led0/brightness
sudo chmod 770 /sys/class/leds/led0/brightness