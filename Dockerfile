FROM softprops/lambda-rust

COPY Cargo.* /code/
COPY src/ /code/src/