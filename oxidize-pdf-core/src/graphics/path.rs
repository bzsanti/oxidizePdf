#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

pub struct PathBuilder {
    commands: Vec<PathCommand>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum PathCommand {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CurveTo(f64, f64, f64, f64, f64, f64),
    ClosePath,
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl PathBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn move_to(mut self, x: f64, y: f64) -> Self {
        self.commands.push(PathCommand::MoveTo(x, y));
        self
    }

    pub fn line_to(mut self, x: f64, y: f64) -> Self {
        self.commands.push(PathCommand::LineTo(x, y));
        self
    }

    pub fn curve_to(mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> Self {
        self.commands
            .push(PathCommand::CurveTo(x1, y1, x2, y2, x3, y3));
        self
    }

    pub fn close(mut self) -> Self {
        self.commands.push(PathCommand::ClosePath);
        self
    }

    pub(crate) fn build(self) -> Vec<PathCommand> {
        self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_cap_values() {
        assert_eq!(LineCap::Butt as u8, 0);
        assert_eq!(LineCap::Round as u8, 1);
        assert_eq!(LineCap::Square as u8, 2);
    }

    #[test]
    fn test_line_cap_equality() {
        assert_eq!(LineCap::Butt, LineCap::Butt);
        assert_ne!(LineCap::Butt, LineCap::Round);
        assert_ne!(LineCap::Round, LineCap::Square);
    }

    #[test]
    fn test_line_cap_debug() {
        let butt = LineCap::Butt;
        let debug_str = format!("{butt:?}");
        assert_eq!(debug_str, "Butt");

        let round = LineCap::Round;
        assert_eq!(format!("{round:?}"), "Round");

        let square = LineCap::Square;
        assert_eq!(format!("{square:?}"), "Square");
    }

    #[test]
    fn test_line_cap_clone() {
        let cap = LineCap::Round;
        let cap_clone = cap;
        assert_eq!(cap, cap_clone);
    }

    #[test]
    fn test_line_cap_copy() {
        let cap = LineCap::Square;
        let cap_copy = cap; // Copy semantics
        assert_eq!(cap, cap_copy);

        // Both should still be usable
        assert_eq!(cap, LineCap::Square);
        assert_eq!(cap_copy, LineCap::Square);
    }

    #[test]
    fn test_line_join_values() {
        assert_eq!(LineJoin::Miter as u8, 0);
        assert_eq!(LineJoin::Round as u8, 1);
        assert_eq!(LineJoin::Bevel as u8, 2);
    }

    #[test]
    fn test_line_join_equality() {
        assert_eq!(LineJoin::Miter, LineJoin::Miter);
        assert_ne!(LineJoin::Miter, LineJoin::Round);
        assert_ne!(LineJoin::Round, LineJoin::Bevel);
    }

    #[test]
    fn test_line_join_debug() {
        let miter = LineJoin::Miter;
        let debug_str = format!("{miter:?}");
        assert_eq!(debug_str, "Miter");

        let round = LineJoin::Round;
        assert_eq!(format!("{round:?}"), "Round");

        let bevel = LineJoin::Bevel;
        assert_eq!(format!("{bevel:?}"), "Bevel");
    }

    #[test]
    fn test_line_join_clone() {
        let join = LineJoin::Bevel;
        let join_clone = join;
        assert_eq!(join, join_clone);
    }

    #[test]
    fn test_line_join_copy() {
        let join = LineJoin::Miter;
        let join_copy = join; // Copy semantics
        assert_eq!(join, join_copy);

        // Both should still be usable
        assert_eq!(join, LineJoin::Miter);
        assert_eq!(join_copy, LineJoin::Miter);
    }

    #[test]
    fn test_path_builder_new() {
        let builder = PathBuilder::new();
        let commands = builder.build();
        assert!(commands.is_empty());
    }

    #[test]
    fn test_path_builder_default() {
        let builder = PathBuilder::default();
        let commands = builder.build();
        assert!(commands.is_empty());
    }

    #[test]
    fn test_path_builder_move_to() {
        let builder = PathBuilder::new().move_to(10.0, 20.0);
        let commands = builder.build();

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PathCommand::MoveTo(x, y) => {
                assert_eq!(*x, 10.0);
                assert_eq!(*y, 20.0);
            }
            _ => panic!("Expected MoveTo command"),
        }
    }

    #[test]
    fn test_path_builder_line_to() {
        let builder = PathBuilder::new().line_to(30.0, 40.0);
        let commands = builder.build();

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PathCommand::LineTo(x, y) => {
                assert_eq!(*x, 30.0);
                assert_eq!(*y, 40.0);
            }
            _ => panic!("Expected LineTo command"),
        }
    }

    #[test]
    fn test_path_builder_curve_to() {
        let builder = PathBuilder::new().curve_to(10.0, 20.0, 30.0, 40.0, 50.0, 60.0);
        let commands = builder.build();

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PathCommand::CurveTo(x1, y1, x2, y2, x3, y3) => {
                assert_eq!(*x1, 10.0);
                assert_eq!(*y1, 20.0);
                assert_eq!(*x2, 30.0);
                assert_eq!(*y2, 40.0);
                assert_eq!(*x3, 50.0);
                assert_eq!(*y3, 60.0);
            }
            _ => panic!("Expected CurveTo command"),
        }
    }

    #[test]
    fn test_path_builder_close() {
        let builder = PathBuilder::new().close();
        let commands = builder.build();

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PathCommand::ClosePath => {}
            _ => panic!("Expected ClosePath command"),
        }
    }

    #[test]
    fn test_path_builder_complex_path() {
        let builder = PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close();

        let commands = builder.build();
        assert_eq!(commands.len(), 5);

        match &commands[0] {
            PathCommand::MoveTo(x, y) => {
                assert_eq!(*x, 0.0);
                assert_eq!(*y, 0.0);
            }
            _ => panic!("Expected MoveTo at index 0"),
        }

        match &commands[1] {
            PathCommand::LineTo(x, y) => {
                assert_eq!(*x, 100.0);
                assert_eq!(*y, 0.0);
            }
            _ => panic!("Expected LineTo at index 1"),
        }

        match &commands[4] {
            PathCommand::ClosePath => {}
            _ => panic!("Expected ClosePath at index 4"),
        }
    }

    #[test]
    fn test_path_builder_bezier_curve() {
        let builder = PathBuilder::new()
            .move_to(0.0, 0.0)
            .curve_to(50.0, 0.0, 100.0, 50.0, 100.0, 100.0)
            .curve_to(100.0, 150.0, 50.0, 200.0, 0.0, 200.0);

        let commands = builder.build();
        assert_eq!(commands.len(), 3);

        match &commands[1] {
            PathCommand::CurveTo(x1, y1, x2, y2, x3, y3) => {
                assert_eq!(*x1, 50.0);
                assert_eq!(*y1, 0.0);
                assert_eq!(*x2, 100.0);
                assert_eq!(*y2, 50.0);
                assert_eq!(*x3, 100.0);
                assert_eq!(*y3, 100.0);
            }
            _ => panic!("Expected CurveTo at index 1"),
        }
    }

    #[test]
    fn test_path_command_debug() {
        let move_cmd = PathCommand::MoveTo(10.0, 20.0);
        let debug_str = format!("{move_cmd:?}");
        assert!(debug_str.contains("MoveTo"));
        assert!(debug_str.contains("10.0"));
        assert!(debug_str.contains("20.0"));

        let line_cmd = PathCommand::LineTo(30.0, 40.0);
        let line_debug = format!("{line_cmd:?}");
        assert!(line_debug.contains("LineTo"));

        let curve_cmd = PathCommand::CurveTo(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let curve_debug = format!("{curve_cmd:?}");
        assert!(curve_debug.contains("CurveTo"));

        let close_cmd = PathCommand::ClosePath;
        let close_debug = format!("{close_cmd:?}");
        assert!(close_debug.contains("ClosePath"));
    }

    #[test]
    fn test_path_command_clone() {
        let move_cmd = PathCommand::MoveTo(10.0, 20.0);
        let move_clone = move_cmd.clone();
        match (move_cmd, move_clone) {
            (PathCommand::MoveTo(x1, y1), PathCommand::MoveTo(x2, y2)) => {
                assert_eq!(x1, x2);
                assert_eq!(y1, y2);
            }
            _ => panic!("Clone failed"),
        }

        let close_cmd = PathCommand::ClosePath;
        let close_clone = close_cmd.clone();
        match close_clone {
            PathCommand::ClosePath => {}
            _ => panic!("Clone failed for ClosePath"),
        }
    }

    #[test]
    fn test_path_builder_empty_path() {
        let builder = PathBuilder::new();
        let commands = builder.build();
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_path_builder_single_command() {
        // Test each command type in isolation
        let move_builder = PathBuilder::new().move_to(5.0, 10.0);
        assert_eq!(move_builder.build().len(), 1);

        let line_builder = PathBuilder::new().line_to(15.0, 20.0);
        assert_eq!(line_builder.build().len(), 1);

        let curve_builder = PathBuilder::new().curve_to(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(curve_builder.build().len(), 1);

        let close_builder = PathBuilder::new().close();
        assert_eq!(close_builder.build().len(), 1);
    }

    #[test]
    fn test_path_builder_negative_coordinates() {
        let builder = PathBuilder::new()
            .move_to(-10.0, -20.0)
            .line_to(-30.0, -40.0)
            .curve_to(-1.0, -2.0, -3.0, -4.0, -5.0, -6.0);

        let commands = builder.build();
        assert_eq!(commands.len(), 3);

        match &commands[0] {
            PathCommand::MoveTo(x, y) => {
                assert_eq!(*x, -10.0);
                assert_eq!(*y, -20.0);
            }
            _ => panic!("Expected MoveTo"),
        }
    }

    #[test]
    fn test_path_builder_zero_values() {
        let builder = PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(0.0, 0.0)
            .curve_to(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

        let commands = builder.build();
        assert_eq!(commands.len(), 3);

        // All commands should have zero values
        match &commands[0] {
            PathCommand::MoveTo(x, y) => {
                assert_eq!(*x, 0.0);
                assert_eq!(*y, 0.0);
            }
            _ => panic!("Expected MoveTo"),
        }
    }

    #[test]
    fn test_path_builder_large_values() {
        let large_val = 1e6;
        let builder = PathBuilder::new()
            .move_to(large_val, large_val)
            .line_to(large_val * 2.0, large_val * 2.0);

        let commands = builder.build();
        match &commands[0] {
            PathCommand::MoveTo(x, y) => {
                assert_eq!(*x, large_val);
                assert_eq!(*y, large_val);
            }
            _ => panic!("Expected MoveTo"),
        }
    }
}
