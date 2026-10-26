import unittest
from probe import ProbeState


class ProbeStateTest(unittest.TestCase):
    def test_threshold_and_recovery(self):
        state = ProbeState()
        self.assertFalse(state.read())
        for _ in range(2):
            state.record(False)
            self.assertFalse(state.read())
        state.record(False)
        self.assertTrue(state.read())
        state.record(False)
        self.assertTrue(state.read())
        state.record(True)
        self.assertFalse(state.read())
        state.record(False)
        self.assertFalse(state.read())
        self.assertEqual(state.failures, 1)


if __name__ == '__main__':
    unittest.main()
