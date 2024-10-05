FROM r-base:4.4.1

RUN apt-get -y update && apt-get -y install git

RUN git clone https://github.com/h2oai/db-benchmark.git

RUN R -e "install.packages(\"data.table\")"

WORKDIR /data

ENTRYPOINT ["Rscript", "/db-benchmark/_data/groupby-datagen.R", "1e7", "1e2", "0", "0"]

CMD ["1e7", "1e2", "0", "0"]

