FROM softprops/lambda-rust:0.3.0-rust-1.45.0

COPY Cargo.* /code/
COPY src/ /code/src/