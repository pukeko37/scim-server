//! Multi-valued attribute container for SCIM resources.
//!
//! This module provides a generic container for handling multi-valued attributes
//! in SCIM resources, with support for primary value tracking and validation.
//!
//! ## Design Principles
//!
//! - **Type Safety**: Generic over the contained value type
//! - **Primary Constraint**: Ensures at most one primary value exists
//! - **Immutable Operations**: Most operations return new instances
//! - **SCIM Compliance**: Follows SCIM 2.0 multi-valued attribute patterns
//!
//! ## Usage Pattern
//!
//! ```rust
//! use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create multi-valued email addresses
//!     let emails = vec![
//!         EmailAddress::new_simple("work@example.com".to_string())?,
//!         EmailAddress::new_simple("personal@example.com".to_string())?,
//!     ];
//!
//!     let multi_emails = MultiValuedAttribute::new(emails)?;
//!
//!     // Set primary email
//!     let with_primary = multi_emails.with_primary(0)?;
//!
//!     // Access primary email
//!     if let Some(primary) = with_primary.primary() {
//!         println!("Primary email: {}", primary.value());
//!     }
//!     Ok(())
//! }
//! ```

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A generic container for multi-valued SCIM attributes.
///
/// This type provides type-safe handling of multi-valued attributes with
/// support for designating one value as primary. It enforces the SCIM
/// constraint that at most one value can be marked as primary.
///
/// # Type Parameters
///
/// * `T` - The type of values contained in the multi-valued attribute
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let emails = vec![
///         EmailAddress::new_simple("work@example.com".to_string())?,
///         EmailAddress::new_simple("personal@example.com".to_string())?,
///     ];
///
///     let multi_emails = MultiValuedAttribute::new(emails)?;
///     assert_eq!(multi_emails.len(), 2);
///     assert!(multi_emails.primary().is_none());
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiValuedAttribute<T> {
    /// The collection of values
    values: Vec<T>,
    /// Index of the primary value, if any
    primary_index: Option<usize>,
}

impl<T> MultiValuedAttribute<T> {
    /// Creates a new multi-valued attribute from a collection of values.
    ///
    /// # Arguments
    ///
    /// * `values` - Vector of values to store
    ///
    /// # Returns
    ///
    /// * `Ok(MultiValuedAttribute<T>)` - Successfully created multi-valued attribute
    /// * `Err(ValidationError)` - If the input is invalid (e.g., empty vector)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_emails = MultiValuedAttribute::new(emails)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(values: Vec<T>) -> ValidationResult<Self> {
        if values.is_empty() {
            return Err(ValidationError::custom(
                "Multi-valued attribute cannot be empty",
            ));
        }

        Ok(Self {
            values,
            primary_index: None,
        })
    }

    /// Creates a new multi-valued attribute with a single value.
    ///
    /// # Arguments
    ///
    /// * `value` - Single value to store
    ///
    /// # Returns
    ///
    /// A multi-valued attribute containing the single value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let email = EmailAddress::new_simple("user@example.com".to_string())?;
    ///     let multi_email = MultiValuedAttribute::single(email);
    ///     assert_eq!(multi_email.len(), 1);
    ///     Ok(())
    /// }
    /// ```
    pub fn single(value: T) -> Self {
        Self {
            values: vec![value],
            primary_index: None,
        }
    }

    /// Creates a new multi-valued attribute with a single primary value.
    ///
    /// # Arguments
    ///
    /// * `value` - Single value to store as primary
    ///
    /// # Returns
    ///
    /// A multi-valued attribute containing the single primary value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let email = EmailAddress::new_simple("primary@example.com".to_string())?;
    ///     let multi_email = MultiValuedAttribute::single_primary(email);
    ///     assert!(multi_email.primary().is_some());
    ///     Ok(())
    /// }
    /// ```
    pub fn single_primary(value: T) -> Self {
        Self {
            values: vec![value],
            primary_index: Some(0),
        }
    }

    /// Creates an empty multi-valued attribute for internal use.
    ///
    /// This method bypasses validation and is intended for internal use
    /// where empty collections are temporarily needed during construction.
    ///
    /// # Returns
    ///
    /// An empty multi-valued attribute
    pub(crate) fn empty() -> Self {
        Self {
            values: Vec::new(),
            primary_index: None,
        }
    }

    /// Returns the number of values in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     assert_eq!(multi_attr.len(), 2);
    ///     Ok(())
    /// }
    /// ```
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the collection is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// let multi_attr = MultiValuedAttribute::single(
    ///     EmailAddress::new_simple("test@example.com".to_string()).unwrap()
    /// );
    /// assert!(!multi_attr.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns a reference to the primary value, if one is set.
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - Reference to the primary value
    /// * `None` - No primary value is set
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?.with_primary(0)?;
    ///     if let Some(primary) = multi_attr.primary() {
    ///         println!("Primary value found");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn primary(&self) -> Option<&T> {
        self.primary_index.and_then(|index| self.values.get(index))
    }

    /// Returns the index of the primary value, if one is set.
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - Index of the primary value
    /// * `None` - No primary value is set
    pub fn primary_index(&self) -> Option<usize> {
        self.primary_index
    }

    /// Returns a reference to all values in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     for value in multi_attr.values() {
    ///         println!("Value: {:?}", value);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// Returns a reference to the value at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - Reference to the value at the index
    /// * `None` - Index is out of bounds
    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    /// Creates a new multi-valued attribute with the specified value set as primary.
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the value to set as primary
    ///
    /// # Returns
    ///
    /// * `Ok(MultiValuedAttribute<T>)` - New instance with primary value set
    /// * `Err(ValidationError)` - If the index is out of bounds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let with_primary = multi_attr.with_primary(0)?;
    ///     assert!(with_primary.primary().is_some());
    ///     Ok(())
    /// }
    /// ```
    pub fn with_primary(mut self, index: usize) -> ValidationResult<Self> {
        if index >= self.values.len() {
            return Err(ValidationError::custom(format!(
                "Primary index {} is out of bounds for collection of size {}",
                index,
                self.values.len()
            )));
        }

        self.primary_index = Some(index);
        Ok(self)
    }

    /// Creates a new multi-valued attribute with no primary value set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?.with_primary(0)?;
    ///     let without_primary = multi_attr.without_primary();
    ///     assert!(without_primary.primary().is_none());
    ///     Ok(())
    /// }
    /// ```
    pub fn without_primary(mut self) -> Self {
        self.primary_index = None;
        self
    }

    /// Creates a new multi-valued attribute with an additional value.
    ///
    /// # Arguments
    ///
    /// * `value` - Value to add to the collection
    ///
    /// # Returns
    ///
    /// A new multi-valued attribute with the added value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let new_email = EmailAddress::new_simple("personal@example.com".to_string())?;
    ///     let with_value = multi_attr.with_value(new_email);
    ///     assert_eq!(with_value.len(), 2);
    ///     Ok(())
    /// }
    /// ```
    pub fn with_value(mut self, value: T) -> Self {
        self.values.push(value);
        self
    }

    /// Creates a new multi-valued attribute with an additional primary value.
    ///
    /// This method adds the value and sets it as the primary value.
    ///
    /// # Arguments
    ///
    /// * `value` - Value to add as primary
    ///
    /// # Returns
    ///
    /// A new multi-valued attribute with the added primary value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let new_email = EmailAddress::new_simple("primary@example.com".to_string())?;
    ///     let with_primary = multi_attr.with_primary_value(new_email.clone());
    ///     assert_eq!(with_primary.primary(), Some(&new_email));
    ///     Ok(())
    /// }
    /// ```
    pub fn with_primary_value(mut self, value: T) -> Self {
        self.values.push(value);
        self.primary_index = Some(self.values.len() - 1);
        self
    }

    /// Returns an iterator over the values in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     for value in multi_attr.iter() {
    ///         println!("Value: {:?}", value);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.values.iter()
    }

    /// Validates that at most one value is marked as primary.
    ///
    /// This method is primarily used internally to ensure invariants
    /// are maintained during construction and modification.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Validation passed
    /// * `Err(ValidationError)` - Multiple primary values detected
    pub fn validate_single_primary(&self) -> ValidationResult<()> {
        if let Some(index) = self.primary_index {
            if index >= self.values.len() {
                return Err(ValidationError::custom(
                    "Primary index points to non-existent value",
                ));
            }
        }
        Ok(())
    }

    /// Finds the first value that matches the given predicate.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function to test each value
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - Reference to the first matching value
    /// * `None` - No value matches the predicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let found = multi_attr.find(|value| value.value().contains("work"));
    ///     assert!(found.is_some());
    ///     Ok(())
    /// }
    /// ```
    pub fn find<P>(&self, predicate: P) -> Option<&T>
    where
        P: Fn(&T) -> bool,
    {
        self.values.iter().find(|&value| predicate(value))
    }

    /// Returns all values that satisfy the given predicate.
    /// Filters values that match the given predicate.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function to test each value
    ///
    /// # Returns
    ///
    /// An iterator over references to matching values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let work_values = multi_attr.filter(|v| v.value().contains("work"));
    ///     assert_eq!(work_values.len(), 1);
    ///     Ok(())
    /// }
    /// ```
    pub fn filter<P>(&self, predicate: P) -> Vec<&T>
    where
        P: Fn(&T) -> bool,
    {
        self.values
            .iter()
            .filter(|&value| predicate(value))
            .collect()
    }

    /// Converts the multi-valued attribute into a vector of values.
    ///
    /// This consumes the multi-valued attribute and returns the underlying vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let emails = vec![
    ///         EmailAddress::new_simple("work@example.com".to_string())?,
    ///         EmailAddress::new_simple("personal@example.com".to_string())?,
    ///     ];
    ///     let multi_attr = MultiValuedAttribute::new(emails)?;
    ///     let values_vec = multi_attr.into_values();
    ///     assert_eq!(values_vec.len(), 2);
    ///     Ok(())
    /// }
    /// ```
    pub fn into_values(self) -> Vec<T> {
        self.values
    }
}

impl<T> Default for MultiValuedAttribute<T> {
    /// Creates an empty multi-valued attribute.
    ///
    /// Note: This creates an empty collection which may not be valid
    /// for all use cases. Use `new()` for validated construction.
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> From<Vec<T>> for MultiValuedAttribute<T> {
    /// Creates a multi-valued attribute from a vector of values.
    ///
    /// This bypasses validation and should be used carefully.
    /// Consider using `new()` for validated construction.
    fn from(values: Vec<T>) -> Self {
        Self {
            values,
            primary_index: None,
        }
    }
}

impl<T> From<T> for MultiValuedAttribute<T> {
    /// Creates a multi-valued attribute from a single value.
    fn from(value: T) -> Self {
        Self::single(value)
    }
}

impl<T> IntoIterator for MultiValuedAttribute<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    /// Creates an iterator that yields owned values.
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a MultiValuedAttribute<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    /// Creates an iterator that yields borrowed values.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: fmt::Display> fmt::Display for MultiValuedAttribute<T> {
    /// Formats the multi-valued attribute for display.
    ///
    /// Shows the number of values and indicates which one is primary.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.primary() {
            Some(primary) => write!(
                f,
                "MultiValuedAttribute({} values, primary: {})",
                self.len(),
                primary
            ),
            None => write!(f, "MultiValuedAttribute({} values)", self.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestValue {
        id: String,
        value_type: String,
    }

    impl fmt::Display for TestValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {}", self.value_type, self.id)
        }
    }

    fn create_test_values() -> Vec<TestValue> {
        vec![
            TestValue {
                id: "1".to_string(),
                value_type: "work".to_string(),
            },
            TestValue {
                id: "2".to_string(),
                value_type: "home".to_string(),
            },
            TestValue {
                id: "3".to_string(),
                value_type: "other".to_string(),
            },
        ]
    }

    #[test]
    fn test_new_valid() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        assert_eq!(multi_attr.len(), 3);
        assert!(!multi_attr.is_empty());
        assert!(multi_attr.primary().is_none());
        assert_eq!(multi_attr.values(), &values);
    }

    #[test]
    fn test_new_empty_fails() {
        let result = MultiValuedAttribute::<TestValue>::new(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_single() {
        let value = TestValue {
            id: "1".to_string(),
            value_type: "work".to_string(),
        };
        let multi_attr = MultiValuedAttribute::single(value.clone());

        assert_eq!(multi_attr.len(), 1);
        assert!(multi_attr.primary().is_none());
        assert_eq!(multi_attr.get(0), Some(&value));
    }

    #[test]
    fn test_single_primary() {
        let value = TestValue {
            id: "1".to_string(),
            value_type: "work".to_string(),
        };
        let multi_attr = MultiValuedAttribute::single_primary(value.clone());

        assert_eq!(multi_attr.len(), 1);
        assert_eq!(multi_attr.primary(), Some(&value));
        assert_eq!(multi_attr.primary_index(), Some(0));
    }

    #[test]
    fn test_with_primary() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone())
            .unwrap()
            .with_primary(1)
            .unwrap();

        assert_eq!(multi_attr.primary(), Some(&values[1]));
        assert_eq!(multi_attr.primary_index(), Some(1));
    }

    #[test]
    fn test_with_primary_invalid_index() {
        let values = create_test_values();
        let result = MultiValuedAttribute::new(values).unwrap().with_primary(10);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));
    }

    #[test]
    fn test_without_primary() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values)
            .unwrap()
            .with_primary(0)
            .unwrap()
            .without_primary();

        assert!(multi_attr.primary().is_none());
        assert_eq!(multi_attr.primary_index(), None);
    }

    #[test]
    fn test_with_value() {
        let values = create_test_values();
        let new_value = TestValue {
            id: "4".to_string(),
            value_type: "mobile".to_string(),
        };

        let multi_attr = MultiValuedAttribute::new(values)
            .unwrap()
            .with_value(new_value.clone());

        assert_eq!(multi_attr.len(), 4);
        assert_eq!(multi_attr.get(3), Some(&new_value));
    }

    #[test]
    fn test_with_primary_value() {
        let values = create_test_values();
        let new_value = TestValue {
            id: "4".to_string(),
            value_type: "mobile".to_string(),
        };

        let multi_attr = MultiValuedAttribute::new(values)
            .unwrap()
            .with_primary_value(new_value.clone());

        assert_eq!(multi_attr.len(), 4);
        assert_eq!(multi_attr.primary(), Some(&new_value));
        assert_eq!(multi_attr.primary_index(), Some(3));
    }

    #[test]
    fn test_find() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        let work_value = multi_attr.find(|v| v.value_type == "work");
        assert_eq!(work_value, Some(&values[0]));

        let mobile_value = multi_attr.find(|v| v.value_type == "mobile");
        assert_eq!(mobile_value, None);
    }

    #[test]
    fn test_filter() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        let work_values = multi_attr.filter(|v| v.value_type == "work");
        assert_eq!(work_values, vec![&values[0]]);

        let non_work_values = multi_attr.filter(|v| v.value_type != "work");
        assert_eq!(non_work_values, vec![&values[1], &values[2]]);
    }

    #[test]
    fn test_into_values() {
        let values = create_test_values();
        let expected = values.clone();
        let multi_attr = MultiValuedAttribute::new(values).unwrap();

        let extracted_values = multi_attr.into_values();
        assert_eq!(extracted_values, expected);
    }

    #[test]
    fn test_iter() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        let collected: Vec<&TestValue> = multi_attr.iter().collect();
        let expected: Vec<&TestValue> = values.iter().collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn test_into_iter_owned() {
        let values = create_test_values();
        let expected = values.clone();
        let multi_attr = MultiValuedAttribute::new(values).unwrap();

        let collected: Vec<TestValue> = multi_attr.into_iter().collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn test_into_iter_borrowed() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        let collected: Vec<&TestValue> = (&multi_attr).into_iter().collect();
        let expected: Vec<&TestValue> = values.iter().collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn test_validate_single_primary() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values)
            .unwrap()
            .with_primary(1)
            .unwrap();

        assert!(multi_attr.validate_single_primary().is_ok());
    }

    #[test]
    fn test_validate_single_primary_invalid_index() {
        let values = create_test_values();
        let mut multi_attr = MultiValuedAttribute::new(values).unwrap();
        // Manually set invalid primary index for testing
        multi_attr.primary_index = Some(10);

        assert!(multi_attr.validate_single_primary().is_err());
    }

    #[test]
    fn test_default() {
        let multi_attr = MultiValuedAttribute::<TestValue>::default();
        assert!(multi_attr.is_empty());
        assert_eq!(multi_attr.len(), 0);
    }

    #[test]
    fn test_from_vec() {
        let values = create_test_values();
        let multi_attr: MultiValuedAttribute<TestValue> =
            MultiValuedAttribute::from(values.clone());

        assert_eq!(multi_attr.len(), 3);
        assert_eq!(multi_attr.values(), &values);
    }

    #[test]
    fn test_from_single_value() {
        let value = TestValue {
            id: "1".to_string(),
            value_type: "work".to_string(),
        };
        let multi_attr = MultiValuedAttribute::from(value.clone());

        assert_eq!(multi_attr.len(), 1);
        assert_eq!(multi_attr.get(0), Some(&value));
    }

    #[test]
    fn test_display() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values).unwrap();

        let display_str = format!("{}", multi_attr);
        assert!(display_str.contains("MultiValuedAttribute(3 values)"));

        let with_primary = multi_attr.with_primary(0).unwrap();
        let primary_display = format!("{}", with_primary);
        assert!(primary_display.contains("primary: work: 1"));
    }

    #[test]
    fn test_get_valid_index() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values.clone()).unwrap();

        assert_eq!(multi_attr.get(0), Some(&values[0]));
        assert_eq!(multi_attr.get(1), Some(&values[1]));
        assert_eq!(multi_attr.get(2), Some(&values[2]));
    }

    #[test]
    fn test_get_invalid_index() {
        let values = create_test_values();
        let multi_attr = MultiValuedAttribute::new(values).unwrap();

        assert_eq!(multi_attr.get(10), None);
    }

    #[test]
    fn test_empty() {
        let multi_attr = MultiValuedAttribute::<TestValue>::empty();
        assert!(multi_attr.is_empty());
        assert_eq!(multi_attr.len(), 0);
        assert!(multi_attr.primary().is_none());
    }
}
