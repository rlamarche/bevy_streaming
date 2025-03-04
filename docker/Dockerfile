# Stage 1: Build dependencies
FROM ubuntu:24.04 AS builder

# Update package lists and install necessary packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    build-essential pkg-config \
    libssl-dev \
    libvulkan-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-nice  \
    gstreamer1.0-tools \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-good1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    libasound2-dev \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"


# Set working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./
# Empty lib
RUN mkdir ./src && touch ./src/lib.rs
# Empty example
RUN mkdir -p ./examples/simple && echo 'fn main() {}' > ./examples/simple/main.rs

# Install Rust dependencies
RUN cargo build --locked --release --examples --target x86_64-unknown-linux-gnu

# Copy source code
COPY src src
COPY examples examples
# Need to touch files after erasing them to have them build again
RUN touch ./examples/simple/main.rs && touch ./src/lib.rs

# Build the application
RUN cargo build --locked --release --examples --target x86_64-unknown-linux-gnu

# Stage 2: Final image
FROM ubuntu:24.04

# Install necessary runtime dependencies (only what's needed for the app to run)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    libvulkan1 \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    # vaapi encoders    
    gstreamer1.0-vaapi \
    gstreamer1.0-libav \
    gstreamer1.0-nice  \
    gstreamer1.0-tools \
    libasound2t64 \
    # make vulkan backend available for mesa
    mesa-vulkan-drivers \
    # intel va encoder driver
    intel-media-va-driver \
    # useful to check vulkan drivers
    vulkan-tools \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy only the necessary built application from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/examples/simple .
COPY assets /app/assets

# Define the entrypoint
CMD ["./simple"]
