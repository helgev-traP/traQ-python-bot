# install docker
apt update
apt install ca-certificates curl gnupg lsb-release -y
mkdir -p /etc/apt/keyrings/
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
chmod a+r /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list >/dev/null
apt update
apt install docker-ce docker-ce-cli containerd.io -y

# bui;d image and run
docker build . -t bot-py
# docker run --rm --privileged bot-py