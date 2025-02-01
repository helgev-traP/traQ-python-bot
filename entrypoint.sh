#!/bin/sh

dockerd &

sleep 5

docker pull python:latest
/app/traq-python-bot