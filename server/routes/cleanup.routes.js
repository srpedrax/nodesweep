const express = require('express');
const { cleanup } = require('../controllers/cleanup.controller');

const router = express.Router();
router.delete('/', cleanup);

module.exports = router;
