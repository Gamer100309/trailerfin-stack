FROM python:3.12.5
WORKDIR /opt/ytdlp2STRM
COPY . /opt/ytdlp2STRM
ENV AM_I_IN_A_DOCKER_CONTAINER Yes
ENV DOCKER_PORT 5005

RUN apt-get update && \
    apt-get install -y ffmpeg curl unzip && \
    apt-get clean

# Deno installieren (JS-Laufzeit, die yt-dlp fuer YouTubes n-Challenge braucht)
RUN curl -fsSL https://deno.land/install.sh | DENO_INSTALL=/usr/local sh
ENV PATH="/usr/local/bin:${PATH}"

# yt-dlp mit [default]-Extras installieren (bringt die EJS-Challenge-Solver-Scripts mit)
RUN pip install --no-cache-dir --upgrade "yt-dlp[default]" && \
    pip install --no-cache-dir --upgrade -r /opt/ytdlp2STRM/requierments.txt

CMD ["python", "main.py"]
EXPOSE 5000
