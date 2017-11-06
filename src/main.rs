#![feature(plugin, custom_derive, decl_macro)]
#![plugin(rocket_codegen)]
extern crate glt;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde_json;

use rocket::request::LenientForm;
use glt::slack::slash_command::Request;
use glt::{handle_command, Result};

fn main() {
    rocket::ignite()
        .mount("/glt", routes![command_request])
        .launch();
}

#[post("/request", data = "<form>")]
fn command_request(form: LenientForm<Request>) -> Result<rocket_contrib::Json> {
    let data = form.into_inner();
    let json = handle_command(data)?;
    Ok(rocket_contrib::Json(json))
}
