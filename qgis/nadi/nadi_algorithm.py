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

from qgis.PyQt.QtCore import QCoreApplication
from qgis.core import (
    QgsBlockingProcess,
    QgsFeatureSink,
    QgsProcessing,
    QgsProcessingAlgorithm,
    QgsProcessingException,
    QgsProcessingFeedback,
    QgsProcessingParameterBoolean,
    QgsProcessingParameterFeatureSink,
    QgsProcessingParameterFeatureSource,
    QgsProcessingParameterField,
    QgsProcessingUtils,
    QgsRunProcess,
)


class NadiAlgorithm(QgsProcessingAlgorithm):
    """
    This is an example algorithm that takes a vector layer and
    creates a new identical one.

    It is meant to be used as an example of how to create your own
    algorithms and explain methods and variables used to do it. An
    algorithm like this will be available in all elements, and there
    is not need for additional work.

    All Processing algorithms should extend the QgsProcessingAlgorithm
    class.
    """

    # Constants used to refer to parameters and outputs. They will be
    # used when calling the algorithm from another algorithm, or when
    # calling from the QGIS console.

    CONNECTIONS = 'CONNECTIONS'
    STREAMS = 'STREAMS'
    POINTS = 'POINTS'
    SIMPLIFY = 'SIMPLIFY'
    STREAMS_ID = 'STREAMS_ID'
    POINTS_ID = 'POINTS_ID'

    def initAlgorithm(self, config):
        """
        Here we define the inputs and output of the algorithm, along
        with some other properties.
        """

        # We add the input vector features source. It can have any kind of
        # geometry.
        self.addParameter(
            QgsProcessingParameterFeatureSource(
                self.STREAMS,
                self.tr('Streams Network'),
                [QgsProcessing.TypeVectorLine]
            )
        )

        self.addParameter(
            QgsProcessingParameterField(
                self.STREAMS_ID,
                self.tr("Primary Key Field for Streams"),
                None,
                self.STREAMS,
                optional=True
            )
        )
        self.addParameter(
            QgsProcessingParameterFeatureSource(
                self.POINTS,
                self.tr('Node Points'),
                [QgsProcessing.TypeVectorPoint]
            )
        )

        self.addParameter(
            QgsProcessingParameterField(
                self.POINTS_ID,
                self.tr("Primary Key Field for Node Points"),
                None,
                self.POINTS,
                optional=True
            )
        )
        
        self.addParameter(
            QgsProcessingParameterBoolean(
                self.SIMPLIFY,
                self.tr('Simplify Connections'),
                False
            )
        )

        # We add a feature sink in which to store our processed features (this
        # usually takes the form of a newly created vector layer when the
        # algorithm is run in QGIS).
        self.addParameter(
            QgsProcessingParameterFeatureSink(
                self.CONNECTIONS,
                self.tr('Output Network')
            )
        )

    def processAlgorithm(self, parameters, context, feedback):
        """
        Here is where the processing itself takes place.
        """

        # Retrieve the feature source and sink. The 'dest_id' variable is used
        # to uniquely identify the feature sink, and must be included in the
        # dictionary returned by the processAlgorithm function.
        streams = self.parameterAsCompatibleSourceLayerPathAndLayerName(
            parameters, self.STREAMS, context, ["gpkg"]
        )
        points = self.parameterAsCompatibleSourceLayerPathAndLayerName(
            parameters, self.POINTS, context, ["gpkg"]
        )
        connection = self.parameterAsOutputLayer(
            parameters, self.CONNECTIONS, context
        )
        simplify = self.parameterAsBool(
            parameters, self.SIMPLIFY, context
        )

        if connection.startswith("memory:"):
            # this does give the temp path, but doesn't connect it with the output layer.
            connection = QgsProcessingUtils.generateTempFilename("connections.gpkg", context)
        elif connection.startswith("ogr:"):
            feedback.reportError("Please use a save to file dialogue.")
            return {self.CONNECTIONS: ""}
        
        feedback.pushInfo("Running Nadi Command:")
        # main command, ignore spatial reference and verbose for progress
        cmd = ["nadi", "connection", "-i", "-v"]
        # add the input layers information
        if simplify:
            cmd += ["-c"]
        cmd += [":".join(points), ":".join(streams), "-o", connection]
        feedback.pushCommandInfo(" ".join(cmd))

        feedback.pushInfo("Output:")


        def stdout_handlr(bytes_array):
            lines = stdout_handlr._buffer + bytes_array.data().decode("utf-8")
            if not lines.endswith('\n'):
                try:
                    lines, stdout_handlr._buffer = lines.rsplit('\n', maxsplit=1)
                except ValueError:
                    stdout_handlr._buffer = lines
                    return
            for line in lines.strip().split('\n'):
                try:
                    label, progress = line.strip().split(":", maxsplit=1)
                    if label != stdout_handlr._curr:
                        feedback.setProgressText(label)
                        stdout_handlr._curr = label
                    feedback.setProgress(int(progress))
                except ValueError:
                    feedback.pushInfo(line)

        def stderr_handlr(bytes_array):
            lines = stderr_handlr._buffer + bytes_array.data().decode("utf-8")
            if not lines.endswith('\n'):
                try:
                    lines, stderr_handlr._buffer = lines.rsplit('\n', maxsplit=1)
                except ValueError:
                    stderr_handlr._buffer = lines
                    return
            for line in lines.strip().split('\n'):
                feedback.pushWarning(line)
        stdout_handlr._buffer = ''
        stderr_handlr._buffer = ''
        stdout_handlr._curr = ''

        proc = QgsBlockingProcess("nadi", cmd[1:])
        proc.setStdOutHandler(stdout_handlr)
        proc.setStdErrHandler(stderr_handlr)

        res = proc.run(feedback)

        if feedback.isCanceled():
            feedback.pushInfo("Cancelled")
        elif res != 0:
            feedback.reportError("Error")
        else:
            feedback.pushInfo("Completed")

        # Return the results of the algorithm. In this case our only result is
        # the feature sink which contains the processed features, but some
        # algorithms may return multiple feature sinks, calculated numeric
        # statistics, etc. These should all be included in the returned
        # dictionary, with keys matching the feature corresponding parameter
        # or output names.
        return {self.CONNECTIONS: connection}

    def name(self):
        """
        Returns the algorithm name, used for identifying the algorithm. This
        string should be fixed for the algorithm, and must not be localised.
        The name should be unique within each provider. Names should contain
        lowercase alphanumeric characters only and no spaces or other
        formatting characters.
        """
        return 'Nadi Connections'

    def displayName(self):
        """
        Returns the translated algorithm name, which should be used for any
        user-visible display of the algorithm name.
        """
        return self.tr(self.name())

    def group(self):
        """
        Returns the name of the group this algorithm belongs to. This string
        should be localised.
        """
        return self.tr(self.groupId())

    def groupId(self):
        """
        Returns the unique ID of the group this algorithm belongs to. This
        string should be fixed for the algorithm, and must not be localised.
        The group id should be unique within each provider. Group id should
        contain lowercase alphanumeric characters only and no spaces or other
        formatting characters.
        """
        return 'Vector'

    def tr(self, string):
        return QCoreApplication.translate('Processing', string)

    def createInstance(self):
        return NadiAlgorithm()
