"""Point location methods. Each class is built from a Solution and offers ``locate(theta) -> int``
(-1 if theta is outside every region) and ``locate_many(thetas) -> ndarray`` (GIL released)."""

from ._rpopt import (  # noqa: F401
    BinarySearchTree,
    DescriptorFunction,
    HashTable,
    LinearMachine,
    MultiwayTree,
    SequentialSearch,
    Yann,
)

__all__ = [
    "SequentialSearch",
    "BinarySearchTree",
    "MultiwayTree",
    "LinearMachine",
    "DescriptorFunction",
    "HashTable",
    "Yann",
]
