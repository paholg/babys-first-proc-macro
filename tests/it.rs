#![allow(dead_code)]
use subenum::subenum;

#[subenum(Dog, Small)]
#[derive(Debug, Copy, Clone)]
enum Canis {
    Wolf,
    #[subenum(Dog)]
    GermanShephard,
    #[subenum(Dog)]
    Boxer,
    #[subenum(Dog)]
    GolderRetriever,
    Coyote,
    #[subenum(Dog, Small)]
    Westie,
}

#[test]
fn test_dog() {
    let canis = Canis::GermanShephard;
    let dog = Dog::try_from(canis).unwrap();
    let canis2 = Canis::from(dog);

    assert_eq!(dog, canis2);
}
