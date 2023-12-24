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
 This script initializes the plugin, making it known to QGIS.
"""

__author__ = 'Gaurav Atreya'
__date__ = '2023-12-21'
__copyright__ = '(C) 2023 by Gaurav Atreya'


# noinspection PyPep8Naming
def classFactory(iface):  # pylint: disable=invalid-name
    """Load Nadi class from file Nadi.

    :param iface: A QGIS interface instance.
    :type iface: QgsInterface
    """
    #
    from .nadi import NadiPlugin
    return NadiPlugin()