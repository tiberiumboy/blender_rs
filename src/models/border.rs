use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub min: f32,
    pub max: f32,
}

// In the python script, this Window values gets assigned to border of scn.render.border_*
// Here - I'm calling it as window instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Border {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32
}

impl Border {
    pub fn get_x_window(&self) -> Window {
        Window { min: self.min_x, max: self.max_x }
    }

    pub fn get_y_window(&self) -> Window {
        Window { min: self.min_y, max: self.max_y }
    }

    pub fn get(&self) -> (Window, Window) {
        (self.get_x_window(), self.get_y_window())
    }
}

impl Default for Border {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            max_x: 1.0,
            min_y: 0.0,
            max_y: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assure_serialize_deserialize_succeed() {
        let obj = Border::default();
        let serialize = serde_json::to_string(&obj);
        assert!(serialize.is_ok());
        let content = serialize.unwrap();   // this prints out "{"min_x":0.0,"max_x":1.0,"min_y":0.0,"max_y": 1.0}"

        let deserialize: Result<Border, _> = serde_json::from_str(&content);
        assert!(deserialize.is_ok());
    }

    #[test]
    fn assure_get_windows_success() {
        let src = Border::default();

        let x = src.get_x_window();
        assert_eq!(x.min, 0.0);
        assert_eq!(x.max, 1.0);

        let y = src.get_y_window();
        assert_eq!(y.min, 0.0);
        assert_eq!(y.max, 1.0);

        let (x1, y1) = src.get();
        assert_eq!(x, x1);
        assert_eq!(y, y1);
    }
}
