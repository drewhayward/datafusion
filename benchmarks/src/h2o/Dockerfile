FROM r-base:4.4.1

RUN apt-get -y update && apt-get -y install git

RUN git clone https://github.com/h2oai/db-benchmark.git

WORKDIR /db-benchmark

RUN R -e "install.packages(\"data.table\")"



