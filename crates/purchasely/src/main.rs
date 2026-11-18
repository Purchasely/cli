fn main() {
    purchasely::main(purchasely::Program {
        name: "purchasely",
        version: env!("CARGO_PKG_VERSION"),
        host: "https://cli.purchasely.io",
        ext: None,
    });
}
