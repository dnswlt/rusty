pub fn foosen() -> String {
    return "foosen".to_string();
}

struct Square {
    width: f64,
}

struct Rect {
    width: f64,
    height: f64,
}

trait Area {
    fn area(&self) -> f64;
}

impl Area for Square {
    fn area(&self) -> f64 {
        return self.width * self.width;
    }
}

impl Area for Rect {
    fn area(&self) -> f64 {
        return self.width * self.height;
    }
}

#[cfg(test)]
mod tests {
    use crate::oop::{Area, Rect, Square};

    #[test]
    fn foosen_from_mod_oop() {
        assert_eq!(crate::oop::foosen(), "foosen");
    }

    #[test]
    fn vec_of_area_items() {
        // Try to put a Square and a Rect into the same Vec:
        let mut v: Vec<Box<dyn Area>> = Vec::new();
        v.push(Box::new(Square { width: 2.0 }));
        v.push(Box::new(Rect {
            width: 1.0,
            height: 4.0,
        }));
        for a in &v {
            assert_eq!(a.area(), 4.0);
        }
    }
}
