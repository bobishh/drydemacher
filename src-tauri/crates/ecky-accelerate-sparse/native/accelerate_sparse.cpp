#include <Accelerate/Accelerate.h>
#include <chrono>
#include <cstring>

extern "C" {
struct EckyAccelerateSolveEvidence { int status; double factor_ms; double solve_ms; };

EckyAccelerateSolveEvidence ecky_accelerate_sparse_solve(
    int dimension, long nonzero_count, const int *rows, const int *columns,
    const double *values, int rhs_count, const double *rhs, double *solution) {
  EckyAccelerateSolveEvidence evidence{-4, 0.0, 0.0};
  if (dimension <= 0 || nonzero_count <= 0 || rhs_count <= 0) return evidence;
  if (__builtin_available(macOS 15.0, *)) {
    if (BLASSetThreading(BLAS_THREADING_MULTI_THREADED) != 0) return evidence;
  }
  SparseAttributes_t attributes{};
  attributes.kind = SparseSymmetric;
  attributes.triangle = SparseUpperTriangle;
  SparseMatrix_Double matrix = SparseConvertFromCoordinate(
      dimension, dimension, nonzero_count, 1, attributes, rows, columns, values);
  auto started = std::chrono::steady_clock::now();
  SparseOpaqueFactorization_Double factor = SparseFactor(SparseFactorizationCholesky, matrix);
  evidence.factor_ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - started).count();
  if (factor.status != SparseStatusOK) {
    evidence.status = factor.status;
    SparseCleanup(factor);
    SparseCleanup(matrix);
    return evidence;
  }
  std::memcpy(solution, rhs, static_cast<size_t>(dimension) * rhs_count * sizeof(double));
  DenseMatrix_Double dense{dimension, rhs_count, dimension, {}, solution};
  started = std::chrono::steady_clock::now();
  SparseSolve(factor, dense);
  evidence.solve_ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - started).count();
  evidence.status = factor.status;
  SparseCleanup(factor);
  SparseCleanup(matrix);
  return evidence;
}
}
