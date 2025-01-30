FROM python:latest

WORKDIR /app
COPY run.py /app/run.py

# 実行に必要なパッケージをインストール
RUN apt-get update && apt-get install -y --no-install-recommends \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# 実行時にスクリプトを使う
ENTRYPOINT ["python3", "/app/run.py"]
