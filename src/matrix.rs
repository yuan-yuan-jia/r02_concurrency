use crate::{dot_product, Vector};
use anyhow::anyhow;
use std::{
    fmt,
    ops::{Add, AddAssign, Mul},
    sync::mpsc,
    thread,
};

const NUM_THREADS: usize = 4;

// [[1, 2], [1, 2], [1, 2]] => [1, 2, 1, 2, 1, 2]
pub struct Matrix<T> {
    data: Vec<T>,
    row: usize,
    col: usize,
}

pub struct MsgInput<T> {
    idx: usize,
    row: Vector<T>,
    col: Vector<T>,
}

pub struct MsgOutput<T> {
    idx: usize,
    value: T,
}

pub struct Msg<T> {
    input: MsgInput<T>,
    sender: oneshot::Sender<MsgOutput<T>>,
}

impl<T> Mul for Matrix<T>
where
    T: Mul<Output = T> + Copy + Default + Add<Output = T> + AddAssign + Send + 'static,
{
    type Output = anyhow::Result<Matrix<T>>;

    fn mul(self, rhs: Self) -> Self::Output {
        multiply::<T>(&self, &rhs)
    }
}

impl<T: fmt::Debug> Matrix<T> {
    pub fn new(data: impl Into<Vec<T>>, row: usize, col: usize) -> Self {
        Self {
            data: data.into(),
            row,
            col,
        }
    }
}

impl<T> fmt::Display for Matrix<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for i in 0..self.row {
            for j in 0..self.col {
                write!(f, "{}", self.data[i * self.col + j])?;
                if j != self.col - 1 {
                    write!(f, " ")?;
                }
            }
            if i != self.row - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl<T> fmt::Debug for Matrix<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Matrix(row={}, col={}, {})", self.row, self.col, self)
    }
}

pub fn multiply<T>(a: &Matrix<T>, b: &Matrix<T>) -> anyhow::Result<Matrix<T>>
where
    T: Copy + Default + Add<Output = T> + AddAssign + Mul<Output = T> + Send + 'static,
{
    if a.col != b.row {
        return Err(anyhow!("Matrix multiply error: a.col != b.row"));
    }
    let senders = (0..NUM_THREADS)
        .map(|_| {
            let (tx, rx) = mpsc::channel::<Msg<T>>();
            thread::spawn(move || {
                for msg in rx {
                    let value = dot_product(msg.input.row, msg.input.col)?;
                    if let Err(e) = msg.sender.send(MsgOutput {
                        idx: msg.input.idx,
                        value,
                    }) {
                        eprintln!("Send error: {:?}", e);
                    }
                }
                Ok::<_, anyhow::Error>(())
            });

            tx
        })
        .collect::<Vec<_>>();

    let matrix_len = a.row * b.col;
    let mut data = vec![T::default(); matrix_len];
    let mut receivers = Vec::with_capacity(matrix_len);

    for i in 0..a.row {
        for j in 0..b.col {
            let row = Vector::new(&a.data[i * a.col..(i + 1) * a.col]);
            let col_data = b.data[j..]
                .iter()
                .step_by(b.col)
                .copied()
                .collect::<Vec<_>>();

            let col = Vector::new(col_data);
            let idx = i * b.col + j;
            let input = MsgInput { idx, row, col };
            let (tx, rx) = oneshot::channel();
            let msg = Msg { input, sender: tx };
            if let Err(e) = senders[idx % NUM_THREADS].send(msg) {
                eprintln!("Send error: {:?}", e);
            }
            receivers.push(rx);
        }
    }

    // map/reduce: reduce phase
    for rx in receivers {
        let output = rx.recv()?;
        data[output.idx] = output.value;
    }
    Ok(Matrix {
        data,
        row: a.row,
        col: b.col,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiply() -> anyhow::Result<()> {
        let a = Matrix::new([1, 2, 3, 4, 5, 6], 2, 3);
        let b = Matrix::new([1, 2, 3, 4, 5, 6], 3, 2);
        let c = multiply(&a, &b)?;
        assert_eq!(c.col, 2);
        assert_eq!(c.row, 2);
        assert_eq!(c.data, vec![22, 28, 49, 64]);
        assert_eq!(format!("{:?}", c), "Matrix(row=2, col=2, {22 28, 49 64})");

        Ok(())
    }
}
