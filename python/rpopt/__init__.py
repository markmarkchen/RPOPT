"""RPOPT: a Rust port of PPOPT with a PPOPT-compatible Python API.

The module layout mirrors PPOPT (``rpopt.mpqp_program``, ``rpopt.mp_solvers.solve_mpqp``, ...).
Call :func:`install_as_ppopt` to register the same modules under the names ``ppopt.*`` so that
existing PPOPT scripts run unchanged.
"""

import sys as _sys

from . import _rpopt
from ._rpopt import (  # noqa: F401
    BinarySearchTree,
    CriticalRegion,
    DescriptorFunction,
    EventTriggered,
    HashTable,
    LinearMachine,
    MPLP_Program,
    MPQP_Program,
    MultiwayTree,
    QueryResult,
    SequentialSearch,
    Solution,
    SolverOutput,
    Yann,
)
from .mp_solvers.solve_mpqp import mpqp_algorithm, solve_mpqp  # noqa: F401

__version__ = "0.1.0"


def install_as_ppopt():
    """Make ``import ppopt...`` resolve to RPOPT (only for the modules RPOPT provides)."""
    import importlib

    names = [
        "mpqp_program",
        "mplp_program",
        "solution",
        "critical_region",
        "mp_solvers",
        "mp_solvers.solve_mpqp",
        "mp_solvers.solve_mplp",
        "point_location",
    ]
    _sys.modules["ppopt"] = _sys.modules[__name__]
    for n in names:
        _sys.modules["ppopt." + n] = importlib.import_module(f"{__name__}.{n}")
