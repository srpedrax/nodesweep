const express = require('express');
const { scan } = require('../controllers/scan.controller');

const router = express.Router();
router.post('/', scan);

module.exports = router;
