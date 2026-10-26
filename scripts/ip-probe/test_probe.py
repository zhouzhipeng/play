import unittest
from unittest.mock import MagicMock, patch
from probe import should_change_ip


class LiveProbeTest(unittest.TestCase):
    def test_checks_again_after_failure_and_recovery(self):
        with patch('probe.socket.create_connection') as connect:
            connect.side_effect = [TimeoutError('timed out'), MagicMock(), OSError('refused')]
            self.assertTrue(should_change_ip())
            self.assertFalse(should_change_ip())
            self.assertTrue(should_change_ip())
            self.assertEqual(connect.call_count, 3)
            connect.assert_called_with(('ip.zhouzhipeng.com', 80), timeout=5)


if __name__ == '__main__':
    unittest.main()
