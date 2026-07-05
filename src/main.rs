use crate::bvm::BrainfuckVM;

pub mod bvm;
pub mod blc;

fn main() {
    // let code = "+++++ +++++[>+++++ +++++<-] >++++.---.+++++ ++..+++.>>>+++[>+++++\
    //  +++++<-]>++.<<<<+++++ +++.----- ---.+++.----- -.----- ---.>>+++++ +++++.@";
    // let mut bf = BrainfuckVM::new(code.to_string(), 30000);
    // bf.run();

    let code = std::fs::read_to_string("resources/in.bl").unwrap();
    let (c, s) = blc::parse(code);
    println!("{}", c);
    let mut bf = BrainfuckVM::new(c, s);
    bf.run();
}
