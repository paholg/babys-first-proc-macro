#![allow(dead_code)]
use subenum::subenum;

#[subenum(Dog, Small)]
enum Canis {
    Wolf,
    #[subenum(Dog)]
    Boxer,
    #[subenum(Dog)]
    GolderRetriever,
    Coyote,
    #[subenum(Dog, Small)]
    Westie,
}
