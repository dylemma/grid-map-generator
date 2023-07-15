/// Error type meaning some direction could not be interpreted as a `Cardinal`
#[derive(Debug)]
pub struct NonCardinal;

/// Enum for directions parallel to the X and Y axes.
/// Represented as North, South, East, and West,
/// but could be considered the same as Up, Down, Right, and Left.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Cardinal {
    North,
    East,
    South,
    West,
}