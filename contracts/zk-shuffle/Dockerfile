FROM cosmwasm/optimizer:0.17.0 AS base

RUN echo "https://dl-cdn.alpinelinux.org/alpine/v3.21/main" > /etc/apk/repositories && \
    echo "https://dl-cdn.alpinelinux.org/alpine/v3.21/community" >> /etc/apk/repositories && \
    apk add --no-cache git openssh-client

RUN mkdir -p /root/.ssh && chmod 700 /root/.ssh && \
    ssh-keyscan github.com >> /root/.ssh/known_hosts

# Ensure cargo always uses CLI git for submodules and dependencies
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN echo 'export CARGO_NET_GIT_FETCH_WITH_CLI=true' >> /etc/profile \
 && echo 'export CARGO_NET_GIT_FETCH_WITH_CLI=true' >> /root/.bashrc

# Optional: set git config to use SSH instead of HTTPS (for submodules)
RUN git config --global url."ssh://git@github.com/".insteadOf "https://github.com/"

ENTRYPOINT ["/usr/local/bin/optimize.sh"]
