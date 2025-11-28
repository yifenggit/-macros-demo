// use macros::create_func;
// use macros::enum_to_str;
// use macros::find_min;
// use macros::print_result;
// use macros::test;

// use proc_macros::make_answer;
// make_answer!();

// use proc_macros::AnswerFn;

// #[derive(AnswerFn)]
// struct Struct;

// use proc_macros::show_streams;

// // 示例: 基础函数
// #[show_streams]
// fn invoke1() {}
// // out: attr: ""
// // out: item: "fn invoke1() { }"

// // 示例: 带输入参数的属性
// #[show_streams(bar)]
// pub fn invoke2() {}
// // out: attr: "bar"
// // out: item: "fn invoke2() {}"

// // 示例: 输入参数中有多个 token 的
// #[show_streams(multiple => tokens)]
// fn invoke3() {}
// // out: attr: "multiple => tokens"
// // out: item: "fn invoke3() {}"

// // 示例:
// #[show_streams { delimiters }]
// fn invoke4() {}
// // out: attr: "delimiters"
// // out: item: "fn invoke4() {}"

// #[show_streams]
// struct Attribute {

// }

// use proc_macros::HelperAttr;

// #[derive(HelperAttr)]
// struct StructHelperAttr {
//     #[helper]
//     field: (),
// }

// fn main() {
//     invoke2();

//     // println!("{}", answer());

//     assert_eq!(42, answer_fn());

//     enum Status {
//         Active,
//         Paused,
//     }
//     enum_to_str!(Status {
//         Status::Active => "运行中",
//         Status::Paused => "已暂停"
//     });
//     println!("{}", Status::Active.as_str());
//     println!("{}", Status::Paused.as_str());

//     create_func!(hello);

//     hello();

//     print_result!(1 + 1);

//     print_result!({
//         let x = 1u32;

//         x * x + 2 * x - 1
//     });

//     test!(true; and true);

//     println!("The minimum is {}", find_min!(1, 2, 3, 4, 5));

// }

use mymacros::proc_test;

fn main() {
    // println!("Hello, world!");
    // let _ = proc_test();

    macro_rules! print_tt {
        // ($input:tt) => {
        //     println!("The token tree is: {:?}", stringify!($input));
        // };
        ($($tt:tt)*) => { [$(stringify!($tt)),*].len() };
    }
    let a = print_tt!(a b c + 1 2 3);
    println!("{}", a);

    macro_rules! call_func {
        ($p:path) => {
            $p()
        };
    }
    fn foo() {
        println!("foo called");
    }
    call_func!(foo);
}
