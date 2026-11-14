from enum import Enum

from .solve_mpqp import mpqp_algorithm, solve_mpqp


class mplp_solver(Enum):
    Dustin = "1"


def solve_mplp(problem, algorithm=mplp_solver.Dustin):
    """PPOPT's mpLP entry point (a stub there). Here it solves with the combinatorial algorithm."""
    return solve_mpqp(problem, mpqp_algorithm.combinatorial)
