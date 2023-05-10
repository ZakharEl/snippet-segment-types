//! Implementors of InteractiveSegment.
//! Essentially different types of snippet portions like those found in [textmate editor](https://macromates.com/manual/en/snippets) and [visual studio code](https://code.visualstudio.com/docs/editor/userdefinedsnippets#_snippet-syntax) (placeholders, choice, etc).
//! This library does not include parsers
//! This is the responsability of another progam that uses this library so as to enable custom snippet body string syntax
//! This also achieves the state of being unopinionated for parsing a snippet body string into segments
pub use snippet_body::*;
use std::fmt;

/// Text typed in by user.
/// Also serves what visual studio code and textmate describes as tabs and mirrors.
pub struct Placeholder(
	/// Content of placeholder.
	/// Is vec of segments since placeholder can contain not merely just plain text but also things like other placeholders.
	pub Vec<Segment>
);
impl fmt::Display for Placeholder {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Placeholder(segs) = self;
		for seg in segs {
			seg.fmt(f)?;
		}
		Ok(())
	}
}
impl InteractiveSegment for Placeholder {
	fn get_type(&self) -> &str {
		"placeholder"
	}
	fn nested_segments(&self) -> Option<&Vec<Segment>> {
		Some(&self.0)
	}
}
impl Field for Placeholder {
}

/// Choice of text selected by user from a menu of several.
pub struct Choice(
	/// Index of the chosen choice from within the outer vec of the field below.
	pub usize,
	/// Outer vec is the choices whereas the inner vec is the segments within a given choice.
	pub Vec<Vec<Segment>>
);
impl fmt::Display for Choice {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let Choice(choice, choices) = self;
		let choice = if let Some(choice) = choices.get(*choice) {
			choice
		} else {
			return Ok(())
		};
		for seg in choice {
			seg.fmt(f)?;
		}
		Ok(())
	}
}
impl InteractiveSegment for Choice {
	fn get_type(&self) -> &str {
		"choice"
	}
	fn nested_segments(&self) -> Option<&Vec<Segment>> {
		let Choice(choice, choices) = self;
		if let Some(choice) = choices.get(*choice) {
			Some(choice)
		} else {
			None
		}
	}
}
impl Field for Choice {
}

/// Part of the snippet that is filled in by program variables (ie environment variables).
pub struct Variable {
	/// Name of the variable.
	pub name: String,
	/// Value of the variable.
	pub value: String,
	/// Where a variable comes from.
	pub get_from_client: Option<*mut dyn FnMut(&str) -> String>
}
fn get_variable_value(name: &str) -> String {
	if let Ok(value) = std::env::var(name) {
		value
	} else {
		String::new()
	}
}
impl fmt::Display for Variable {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.value.fmt(f)
	}
}
impl InteractiveSegment for Variable {
	fn get_type(&self) -> &str {
		"variable"
	}
}
impl Programic for Variable {
	fn evaluate(&mut self) {
		self.value = if let Some(get_from_client_function) = self.get_from_client {
			unsafe{
				(*get_from_client_function)(&self.name)
			}
		} else {
			get_variable_value(&self.name)
		};
	}
	fn indentifier(&self) -> &String {
		&self.name
	}
}

/// [Shell Code](https://macromates.com/manual/en/snippets#interpolated_shell_code) to run.
/// Output will be the string show/expanded within the snippet
pub struct Code {
	pub code_to_run: String,
	pub output: String
}
impl fmt::Display for Code {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.output.fmt(f)
	}
}
impl InteractiveSegment for Code {
	fn get_type(&self) -> &str {
		"code"
	}
}
impl Programic for Code {
	fn evaluate(&mut self) {
		let options = run_script::ScriptOptions::new();
		let args = vec![];
		let (_, output, _) = run_script::run(&self.code_to_run, &args, &options).unwrap();
		self.output = output;
	}
	fn indentifier(&self) -> &String {
		&self.code_to_run
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::rc::Rc;
	use std::cell::RefCell;
	#[test]
	fn trait_object_casting() {
		let mut snippet = Snippet {
			body: vec![],
			tabs: vec![],
			program_filled_text: vec![],
			references: vec![]
		};
		let placeholder = Placeholder(vec![
			Segment::Text(String::from("hello")),
			Segment::Text(String::from("there!"))
		]);
		let placeholder_rc = Rc::new(RefCell::new(placeholder));
		let placeholder_clone = Rc::clone(&placeholder_rc);
		let tab = Tab {
			num: 1,
			field: placeholder_clone
		};
		let mut code = Code {
			code_to_run: String::from("greet=hi echo no"),
			output: String::new()
		};
		code.evaluate();
		let code_rc = Rc::new(RefCell::new(code));
		let code_clone = Rc::clone(&code_rc);
		snippet.program_filled_text.push(code_clone);
		let placeholder_weak = Rc::downgrade(&placeholder_rc);
		let mut placeholder = Placeholder(vec![
			Segment::Interactive(placeholder_rc)
		]);
		let interactive = &snippet.program_filled_text[0];
		let interactive = &mut *interactive.borrow_mut();
		let interactive: &mut Code = cast_mut_programic(interactive).unwrap();
		interactive.code_to_run = String::from("greet=hi echo yes");
		interactive.evaluate();
		println!("return of code runned: {}", interactive);
		if let Placeholder(ref mut segs) = placeholder {
			if let Segment::Interactive(interactive) = &segs[0] {
				let interactive = &*interactive.borrow_mut();
				let placeholder: &Placeholder = cast_interactive_segment(interactive).unwrap();
				if let None = cast_interactive_segment::<Choice>(interactive) {
					println!("Not a choice!");
				}
				if let Placeholder(ref segs) = placeholder {
					println!("Placeholder length: {}", segs.len());
					println!("Second segment: {}", &segs[1]);
				}
			}
//			segs[0] = Segment::Text(String::from("dud!"));
		}
		let field_rc = tab.field;
		println!("Reference!");
		let field = &*field_rc.borrow();
		let placeholder: &Placeholder = cast_field(field).unwrap();
		println!("Just making sure! {}", placeholder);
		println!("Just making sure again! {}", placeholder);
	}
	#[test]
	fn partial_eq_test() {
		let reference = Reference::Text(String::from("Greetings"), String::from("Hi"));
		let reference = Rc::new(RefCell::new(reference));
		let segment = Segment::Reference(Rc::clone(&reference));
		assert!(*reference.borrow() == segment, "Reference not equal to segment!");
		println!("{}", *reference.borrow() == segment);
	}
	#[test]
	fn trim_empty_test() {
		let field: Rc<RefCell<dyn Field>> = Rc::new(RefCell::new(Placeholder(
			vec![Segment::Text(
				String::from("Hello ")
			)]
		)));
		let mut snip = Snippet {
			body: vec![],
			tabs: vec![Tab {
				num: 1,
				field: Rc::clone(&field)
			}],
			program_filled_text: vec![],
			references: vec![],
		};
		println!("tab 1 count: {}", Rc::strong_count(&snip.tabs[0].field));
		snip.trim_empty_tabs();
		println!("tab 1 count: {}", Rc::strong_count(&snip.tabs[0].field));
		println!("number of tabs: {}", snip.tabs.len());
	}
}
