use sonic_rs::JsonValueMutTrait;
use sonic_rs::{JsonValueTrait, Value, pointer};
use sonic_rs::{from_str, json};

// fn main() {
//     let json = r#"{
//         "name": "Xiaoming",
//         "obj": {},
//         "arr": [],
//         "age": 18,
//         "address": {
//             "city": "Beijing"
//         },
//         "phones": [
//             "+123456"
//         ]
//     }"#;

//     let mut root: Value = from_str(json).unwrap();

//     // get key from value
//     let age = root.get("age").as_i64();
//     assert_eq!(age.unwrap_or_default(), 18);

//     // get by index
//     let first = root["phones"][0].as_str().unwrap();
//     assert_eq!(first, "+123456");

//     // get by pointer
//     let phones = root.pointer(&pointer!["phones", 0]);
//     assert_eq!(phones.as_str().unwrap(), "+123456");

//     // convert to mutable object
//     let obj = root.as_object_mut().unwrap();
//     obj.insert(&"inserted", true);
//     assert!(obj.contains_key(&"inserted"));

//     let mut object = json!({ "A": 65, "B": 66, "C": 67 });
//     *object.get_mut("A").unwrap() = json!({
//         "code": 123,
//         "success": false,
//         "payload": {}
//     });

//     let mut val = json!(["A", "B", "C"]);
//     *val.get_mut(2).unwrap() = json!("D");

//     // serialize
//     assert_eq!(serde_json::to_string(&val).unwrap(), r#"["A","B","D"]"#);
// }

fn main() {
    use std::collections::HashMap;

    // Type inference lets us omit an explicit type signature (which
    // would be `HashMap<String, String>` in this example).
    let mut book_reviews = HashMap::new();
    

    // Review some books.
    book_reviews.insert(
        "Adventures of Huckleberry Finn".to_string(),
        "My favorite book.".to_string(),
    );
    book_reviews.insert(
        "Grimms' Fairy Tales".to_string(),
        "Masterpiece.".to_string(),
    );
    book_reviews.insert(
        "Pride and Prejudice".to_string(),
        "Very enjoyable.".to_string(),
    );
    book_reviews.insert(
        "The Adventures of Sherlock Holmes".to_string(),
        "Eye lyked it alot.".to_string(),
    );

    // Check for a specific one.
    // When collections store owned values (String), they can still be
    // queried using references (&str).
    if !book_reviews.contains_key("Les Misérables") {
        println!(
            "We've got {} reviews, but Les Misérables ain't one.",
            book_reviews.len()
        );
    }

    book_reviews.contains_key("The Adventures of Sherlock Holmes");

    // oops, this review has a lot of spelling mistakes, let's delete it.
    book_reviews.remove("The Adventures of Sherlock Holmes");

    let a = String::from("hellow");

    // Look up the values associated with some keys.
    let to_find = ["Pride and Prejudice", "Alice's Adventure in Wonderland"];
    for &book in &to_find {
        match book_reviews.get(book) {
            Some(review) => println!("{book}: {review}"),
            None => println!("{book} is unreviewed."),
        }
    }

    // Look up the value for a key (will panic if the key is not found).
    println!("Review for Jane: {}", book_reviews["Pride and Prejudice"]);

    // Iterate over everything.
    for (book, review) in &book_reviews {
        println!("{book}: \"{review}\"");
    }
}
