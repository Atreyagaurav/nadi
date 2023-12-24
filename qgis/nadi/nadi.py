# -*- coding: utf-8 -*-

"""
/***************************************************************************
 Nadi
                                 A QGIS plugin
 Nadi (River) connections tool
 Generated by Plugin Builder: http://g-sherman.github.io/Qgis-Plugin-Builder/
                              -------------------
        begin                : 2023-12-21
        copyright            : (C) 2023 by Gaurav Atreya
        email                : allmanpride@gmail.com
 ***************************************************************************/

/***************************************************************************
 *                                                                         *
 *   This program is free software; you can redistribute it and/or modify  *
 *   it under the terms of the GNU General Public License as published by  *
 *   the Free Software Foundation; either version 2 of the License, or     *
 *   (at your option) any later version.                                   *
 *                                                                         *
 ***************************************************************************/
"""

__author__ = 'Gaurav Atreya'
__date__ = '2023-12-21'
__copyright__ = '(C) 2023 by Gaurav Atreya'

# This will get replaced with a git SHA1 when you do a git archive

__revision__ = '$Format:%H$'

import os
import sys
import inspect

from qgis.core import QgsProcessingAlgorithm, QgsApplication
from .nadi_provider import NadiProvider

cmd_folder = os.path.split(inspect.getfile(inspect.currentframe()))[0]

if cmd_folder not in sys.path:
    sys.path.insert(0, cmd_folder)


class NadiPlugin(object):

    def __init__(self):
        self.provider = "Nadi"

    def initProcessing(self):
        """Init Processing provider for QGIS >= 3.8."""
        self.provider = NadiProvider()
        QgsApplication.processingRegistry().addProvider(self.provider)

    def initGui(self):
        self.initProcessing()

    def unload(self):
        QgsApplication.processingRegistry().removeProvider(self.provider)
