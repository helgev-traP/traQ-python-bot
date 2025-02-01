#!/bin/sh

dockerd > /dev/null 2>&1 &

sleep 5

docker pull python:latest
/app/traq-python-bot