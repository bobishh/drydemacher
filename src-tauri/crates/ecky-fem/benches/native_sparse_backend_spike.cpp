#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <string>
#include <tuple>
#include <vector>

#if defined(ECKY_ACCELERATE)
#include <Accelerate/Accelerate.h>
#elif defined(ECKY_CHOLMOD)
#include <cholmod.h>
#else
#error "define ECKY_ACCELERATE or ECKY_CHOLMOD"
#endif

struct Fixture {
  int n;
  int nrhs;
  std::vector<int> row;
  std::vector<int> col;
  std::vector<double> value;
  std::vector<int> upper_row;
  std::vector<int> upper_col;
  std::vector<double> upper_value;
  std::vector<double> rhs;
};

static void skip_comments(std::istream &input) {
  while (input.peek() == '%') {
    std::string line;
    std::getline(input, line);
  }
}

static Fixture read_fixture(const char *matrix_path, const char *rhs_path) {
  std::ifstream matrix(matrix_path);
  std::string banner;
  std::getline(matrix, banner);
  if (banner != "%%MatrixMarket matrix coordinate real general") std::abort();
  skip_comments(matrix);
  int nrows = 0, ncols = 0;
  long nnz = 0;
  matrix >> nrows >> ncols >> nnz;
  if (nrows != ncols) std::abort();
  Fixture fixture{nrows, 0};
  fixture.row.reserve(nnz);
  fixture.col.reserve(nnz);
  fixture.value.reserve(nnz);
  for (long index = 0; index < nnz; ++index) {
    int row = 0, col = 0;
    double value = 0.0;
    matrix >> row >> col >> value;
    --row;
    --col;
    fixture.row.push_back(row);
    fixture.col.push_back(col);
    fixture.value.push_back(value);
    if (row <= col) {
      fixture.upper_row.push_back(row);
      fixture.upper_col.push_back(col);
      fixture.upper_value.push_back(value);
    }
  }
  std::ifstream rhs(rhs_path);
  std::getline(rhs, banner);
  if (banner != "%%MatrixMarket matrix array real general") std::abort();
  skip_comments(rhs);
  int rhs_rows = 0;
  rhs >> rhs_rows >> fixture.nrhs;
  if (rhs_rows != fixture.n) std::abort();
  fixture.rhs.resize(static_cast<size_t>(fixture.n) * fixture.nrhs);
  for (double &value : fixture.rhs) rhs >> value;
  return fixture;
}

static double maximum_relative_residual(const Fixture &fixture,
                                        const std::vector<double> &solution) {
  double maximum = 0.0;
  for (int rhs_index = 0; rhs_index < fixture.nrhs; ++rhs_index) {
    std::vector<double> residual(fixture.n);
    double rhs_squared = 0.0;
    for (int row = 0; row < fixture.n; ++row) {
      const double rhs = fixture.rhs[static_cast<size_t>(rhs_index) * fixture.n + row];
      residual[row] = -rhs;
      rhs_squared += rhs * rhs;
    }
    for (size_t index = 0; index < fixture.value.size(); ++index) {
      residual[fixture.row[index]] +=
          fixture.value[index] * solution[static_cast<size_t>(rhs_index) * fixture.n + fixture.col[index]];
    }
    double residual_squared = 0.0;
    for (double value : residual) residual_squared += value * value;
    maximum = std::max(maximum, std::sqrt(residual_squared) /
                                    std::max(1.0, std::sqrt(rhs_squared)));
  }
  return maximum;
}

static double milliseconds(std::chrono::steady_clock::time_point start) {
  return std::chrono::duration<double, std::milli>(
             std::chrono::steady_clock::now() - start)
      .count();
}

int main(int argc, char **argv) {
  if (argc != 3) return 2;
  Fixture fixture = read_fixture(argv[1], argv[2]);
  std::vector<double> solution = fixture.rhs;
  double factor_ms = 0.0, solve_ms = 0.0;
  const char *backend = nullptr;
  const char *repeat_text = std::getenv("ECKY_BENCH_FACTOR_REPEATS");
  const int factor_repeats = repeat_text ? std::max(1, std::atoi(repeat_text)) : 1;

#if defined(ECKY_ACCELERATE)
  backend = "accelerate-sparse";
  SparseAttributes_t attributes{};
  attributes.kind = SparseSymmetric;
  attributes.triangle = SparseUpperTriangle;
  SparseMatrix_Double matrix = SparseConvertFromCoordinate(
      fixture.n, fixture.n, fixture.upper_value.size(), 1, attributes,
      fixture.upper_row.data(), fixture.upper_col.data(), fixture.upper_value.data());
  SparseOpaqueFactorization_Double factor{};
  auto start = std::chrono::steady_clock::now();
  for (int repeat = 0; repeat < factor_repeats; ++repeat) {
    if (repeat) SparseCleanup(factor);
    factor = SparseFactor(SparseFactorizationCholesky, matrix);
    if (factor.status != SparseStatusOK) return 3;
  }
  factor_ms = milliseconds(start) / factor_repeats;
  DenseMatrix_Double dense{fixture.n, fixture.nrhs, fixture.n, {}, solution.data()};
  start = std::chrono::steady_clock::now();
  SparseSolve(factor, dense);
  solve_ms = milliseconds(start);
  SparseCleanup(factor);
  SparseCleanup(matrix);
#elif defined(ECKY_CHOLMOD)
  backend = "cholmod";
  cholmod_common common;
  cholmod_start(&common);
  common.nthreads_max = 8;
  common.supernodal = CHOLMOD_SUPERNODAL;
  cholmod_triplet *triplet = cholmod_allocate_triplet(
      fixture.n, fixture.n, fixture.upper_value.size(), 1, CHOLMOD_REAL, &common);
  triplet->nnz = fixture.upper_value.size();
  std::memcpy(triplet->i, fixture.upper_row.data(), fixture.upper_row.size() * sizeof(int));
  std::memcpy(triplet->j, fixture.upper_col.data(), fixture.upper_col.size() * sizeof(int));
  std::memcpy(triplet->x, fixture.upper_value.data(), fixture.upper_value.size() * sizeof(double));
  cholmod_sparse *matrix = cholmod_triplet_to_sparse(triplet, triplet->nnz, &common);
  cholmod_factor *factor = nullptr;
  auto start = std::chrono::steady_clock::now();
  for (int repeat = 0; repeat < factor_repeats; ++repeat) {
    if (factor) cholmod_free_factor(&factor, &common);
    factor = cholmod_analyze(matrix, &common);
    if (!factor || !cholmod_factorize(matrix, factor, &common)) return 3;
  }
  factor_ms = milliseconds(start) / factor_repeats;
  cholmod_dense dense{static_cast<size_t>(fixture.n), static_cast<size_t>(fixture.nrhs),
                      static_cast<size_t>(fixture.n * fixture.nrhs),
                      static_cast<size_t>(fixture.n), solution.data(), nullptr, CHOLMOD_REAL,
                      CHOLMOD_DOUBLE};
  start = std::chrono::steady_clock::now();
  cholmod_dense *solved = cholmod_solve(CHOLMOD_A, factor, &dense, &common);
  solve_ms = milliseconds(start);
  std::memcpy(solution.data(), solved->x, solution.size() * sizeof(double));
  cholmod_free_dense(&solved, &common);
  cholmod_free_factor(&factor, &common);
  cholmod_free_sparse(&matrix, &common);
  cholmod_free_triplet(&triplet, &common);
  cholmod_finish(&common);
#endif

  const double residual = maximum_relative_residual(fixture, solution);
  std::cout << "{:backend \"" << backend << "\" :dimension " << fixture.n
            << " :nnz " << fixture.value.size() << " :rhs-count " << fixture.nrhs
            << " :factor-repeats " << factor_repeats
            << " :factor-ms " << factor_ms << " :solve-ms " << solve_ms
            << " :maximum-relative-residual " << residual << "}\n";
  return residual <= 1.0e-8 ? 0 : 4;
}
